use hyper::{Body, Client, Method, Request, Response, StatusCode};
use tokio::io::{split, AsyncWriteExt}; 
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::net::SocketAddr;


pub async fn proxy_services(_req: Request<Body>, remote: SocketAddr ) -> Result<Response<Body>,  hyper::Error> {
    //初始化一个http客户端用来做转发用
    let client= hyper::Client::new();

    match _req.method() {
        &Method::CONNECT => {
            // 对于HTTPS来说开启Connect隧道，制作字节转发
            println!("Received HTTPS request from {}", remote);
            // 这里应该有判断逻辑，选择无证书转发或者有证书解密、转发
            handle_https_without_cert(_req).await
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
    let stream = match TcpStream::connect(target).await {
        Ok(ok) =>  Arc::new(Mutex::new(ok)), 
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
    
    // 4. 后续的字节转发交给更底层的tcp流处理，这里只返回响应头 
    let (mut client_io, mut server_io) = match hyper::upgrade::on(req).await {
        Ok(io) => split(io),
        Err(e) => {
            eprintln!("Error in getting client stream: {}", e);
            let response = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty()).expect("Failed to build response");
            return Ok(response);
        }
    };
    
    let stream_c1 = Arc::clone(&stream);
    // 开新的线程双向转发数据，
    tokio::spawn(async move {
        let mut stream_guard  = stream_c1.lock().await;
        let mut stream1 = &mut *stream_guard;
        // 客户端的请求给目标方，client_io这里是读客户端的
        let _ = tokio::io::copy(&mut client_io, &mut stream1).await;
        let _ = stream1.shutdown().await;
    });

    let stream_c2 = Arc::clone(&stream);
    tokio::spawn(async move {
        let mut stream_guard  = stream_c2.lock().await;
        let mut stream2 = &mut *stream_guard;
        // 目标方的返回给客户端，server_io这里是写给客户端的
        let _ = tokio::io::copy(&mut stream2, &mut server_io).await;
        let _ = server_io.shutdown().await;
    });

    Ok(response)
}


pub async fn handle_https_with_cert(){
    // 再写这个，当客户端安装了证书后。先解密，再转发。
    todo!()
}
