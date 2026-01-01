use hyper::{Body, Client, Uri, Method, Request, Response, StatusCode};
use hyper::service::service_fn;
use hyper_rustls::HttpsConnectorBuilder;
// use hyper_rustls::{HttpsConnector, rustls::{ServerConfig, TlsAcceptor}};
use tokio::io::{AsyncWriteExt, copy_bidirectional};
use log::{info, error, warn};
use tokio::net::TcpStream;
use std::net::SocketAddr;
use std::sync::Arc;
// use rustls::ServerConfig;
use tokio_rustls::{TlsAcceptor, rustls::{ServerConfig, ClientConfig, RootCertStore}};
use crate::utils::cert::{load_cert_from_string, generate_server_cert};


pub async fn proxy_services(_req: Request<Body>, remote: SocketAddr ) -> Result<Response<Body>,  hyper::Error> {
    //初始化一个http客户端用来做转发用
    let client= hyper::Client::new();

    match _req.method() {
        &Method::CONNECT => {
            // 对于HTTPS来说开启Connect隧道，制作字节转发
            println!("Received HTTPS request from {}", remote);
            // 这里应该有判断逻辑，选择无证书转发或者有证书解密、转发
            // handle_https_without_cert(_req).await
            handle_https_with_cert(_req, remote).await
        },
        _ => {
            // 别的HTTP请求解析并转发
            println!("Received HTTP request from {}", remote);
            handle_http_requests(_req, client).await
        }
    }
}


pub async fn handle_http_requests(req: Request<Body>, client: Client<hyper::client::HttpConnector>) -> Result<Response<Body>, hyper::Error> {
    let target = req.uri();
    println!("Forwarding request to target: {}", target);

    let mut request = Request::builder()
                            .method(req.method())
                            .uri(target)
                            .version(req.version());

    let filter_header_list = vec!["Connection" ,"Keep-Alive", "Proxy-Authenticate" ,"Proxy-Authorization",
        "Te" ,"Trailer","Transfer-Encoding" ,"Upgrade"];
    // 解析请求头并过滤打断链接的头
    for (key, value) in req.headers().iter(){
        if filter_header_list.contains(&key.as_str()) {
            continue;
        }
        else{
            request = request.header(key, value);
        }
    }

    // 拷贝请求体，这里采用遮蔽的方式将Builder结构体转换成了Request结构体
    let request = request.body(req.into_body()).expect("Failed to build request");

    // 用client发送请求
    let resp = client.request(request).await?;
    let mut response = Response::builder()
        .status(resp.status())
        .version(resp.version());

    // 复制响应头
    for (key, value) in resp.headers().iter(){
        response = response.header(key, value);
    }

    // 复制响应体
    let response = response.body(resp.into_body()).expect("Failed to build response");

    Ok(response)
}


pub async fn handle_https_without_cert(req:Request<Body>) -> Result<Response<Body>, hyper::Error> {
    // 1. 根据请求找出原来的目标, IP:port格式
    let target = req.uri().authority().unwrap().as_str();
    println!("Establishing tunnel to target: {}", target);
    
    // 2. 连接目标服务器
    let mut stream = match TcpStream::connect(target).await {
        Ok(ok) =>  ok, 
        Err(e) => {
            eprintln!("Failed to connect to target {}: {}", target, e);
            // 返回502 Bad Gateway响应
            let response = Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::empty()).expect("Failed to build response");
            return Ok(response);
        }
    };

    //3. 返回给客户端一个Connection Established响应
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty()).expect("Failed to build response");
    

    let upgraded: hyper::upgrade::OnUpgrade = hyper::upgrade::on(req);

    tokio::spawn(async move{
        match upgraded.await {
            Ok(mut client_stream) => {
                let _ = copy_bidirectional(&mut client_stream, &mut stream).await;
            },
            Err(e) => {
                eprintln!("Upgrade error: {}", e);
            }
        }
    });

    Ok(response)
}


pub async fn handle_https_with_cert(mut req:Request<Body>, remote: SocketAddr) -> Result<Response<Body>, hyper::Error> {
    // 再写这个，当客户端安装了证书后。先解密，再转发。
    let authority = req.uri().authority().unwrap().as_str();
    // 原域名不含端口号时，默认443端口
    let (domain, port) = authority.split_once(':').unwrap_or((authority, "443"));
    println!("建立MITM隧道：{}:{}", domain, port);

    //1. 返回给客户端一个Connection Established响应
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty()).expect("Failed to build response");
        
    // 升级获得更底层stream流的读写能力，即TCP层
    let upgraded:hyper::upgrade::OnUpgrade = hyper::upgrade::on( &mut req);    

    tokio::spawn(async move {
        match upgraded.await {
            Ok(new_stream) =>{
                info!("与客户端 {} 完成升级", remote);
                // move的时候没有将domain移动过来吗？？
                let host = req.uri().host().unwrap();
                // 与客户端建立连接后，可以在这里处理流量
                handle_upgraded_https_traffics(new_stream, host).await;
            },
            Err(e) => {
                error!("升级错误: {}", e);
            }
        }
    });

    Ok(response)
}


async fn handle_upgraded_https_traffics(stream: hyper::upgrade::Upgraded, host:&str) {
    // 1. 加载根证书来为目标服务生成服务器证书
    let (server_ca, server_key) = generate_server_cert(host).unwrap();
    

    // 2. 利用证书和客户端建立连接
    let (certs, pri_key) = load_cert_from_string(server_ca, server_key).unwrap(); // load_cert().unwrap();
    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, pri_key)
        .expect("Failed to create server config");

    // 官方使用文档：https://github.com/rustls/hyper-rustls/blob/main/examples/server.rs
    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));
    let mut client_tls = tls_acceptor.accept(stream).await.unwrap();

    // 3. 代替客户端和真实目标建立https连接
    // let client_config = ClientConfig::builder()
    //         .with_root_certificates(RootCertStore::empty())
    //         .with_no_client_auth();
    // let https_connector = HttpsConnectorBuilder::new()
    //     .with_tls_config(client_config)
    //     .https_or_http()
    //     .enable_http1()
    //     .build();
    // let client = Client::builder().build::<_, Body>(https_connector);


    // 4. 新拉起一个进程在其中完成流量传递。
    tokio::spawn(async move {
       // 代理 <-> 真实服务器：基于 hyper 处理 HTTP 明文
       async fn handle_raw_http_request(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
            // 构建 200 响应，响应体为 Hello from the proxy
            let response = Response::builder()
                .status(StatusCode::OK) // 状态码 200
                .header("Content-Type", "text/plain; charset=utf-8") // 设置响应体格式
                .body(Body::from("Hello from the MIME Proxy")) // 响应体内容
                .unwrap();
            Ok(response)
        }
        let service = service_fn(handle_raw_http_request);

        if let Err(e) = hyper::server::conn::Http::new().
        serve_connection(&mut client_tls, service).
        await{
            error!("Error serving connection: {}", e);
        }
    });


}

