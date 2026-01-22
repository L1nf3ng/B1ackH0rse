use crate::utils::cert;
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::client::conn::http1;
use hyper::server::conn::http1::Builder;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_rustls::{ConfigBuilderExt, HttpsConnectorBuilder};
use hyper_util::client::legacy::Client;
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::convert::Infallible; // 这个错误代表不会出错，所以在函数里即使失败也应该返回Ok(xxx)
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::{
    TlsAcceptor,
    rustls::{ClientConfig, ServerConfig},
};

// hyper升级的官方使用说明：https://hyper.rs/guides/1/upgrading/

pub async fn proxy_services(_req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    match _req.method() {
        &Method::CONNECT => {
            println!("Received HTTPS request from ...");
            // Step1. 先用解密https的方式尝试连接，
            // Step2. 失败后再用纯代理的方式转发。
            // handle_https_without_cert(_req).await
            handle_https_with_cert(_req).await
        }
        _ => {
            println!("Received HTTP request from ...");
            handle_http_requests(_req).await
        }
    }
}

pub async fn handle_http_requests(
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    // 在消费请求之前提取所有需要的信息
    let target_uri = req.uri().clone();
    let method = req.method().clone();
    let version = req.version();
    let headers = req.headers().clone();

    // 获取目标地址
    let authority = req.uri().authority().unwrap().as_str();
    let (domain, port) = authority.split_once(':').unwrap_or((authority, "80"));
    let target = format!("{}:{}", domain, port);

    println!("Forwarding request to target: {}", authority);

    // 连接到目标服务器
    let stream = match TcpStream::connect(target).await {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("Failed to connect to target {}: {}", authority, e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(format!(
                    "Failed to connect to target: {}",
                    e
                ))))
                .unwrap());
        }
    };

    let io = TokioIo::new(stream);

    // 建立 HTTP/1.1 连接
    let (mut sender, _conn) = match http1::handshake::<_, Full<Bytes>>(io).await {
        Ok((sender, conn)) => {
            // 启动连接任务
            tokio::spawn(async move {
                if let Err(err) = conn.await {
                    eprintln!("Connection error: {:?}", err);
                }
            });
            (sender, ())
        }
        Err(e) => {
            eprintln!("Failed to establish HTTP connection: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(format!(
                    "HTTP handshake failed: {}",
                    e
                ))))
                .unwrap());
        }
    };

    // 收集原始请求的请求体（这会消费 req）
    let whole_body = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            eprintln!("Failed to collect request body: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Failed to read request body")))
                .unwrap());
        }
    };

    // 构建转发请求（使用之前提取的信息）
    let mut request_builder = Request::builder()
        .method(method)
        .uri(target_uri)
        .version(version);

    // 过滤不需要的请求头
    let filter_header_list = vec![
        "Connection",
        "Keep-Alive",
        "Proxy-Authenticate",
        "Proxy-Authorization",
        "Te",
        "Trailer",
        "Transfer-Encoding",
        "Upgrade",
    ];

    // 复制请求头（使用之前提取的 headers）
    for (key, value) in headers.iter() {
        if !filter_header_list.contains(&key.as_str()) {
            request_builder = request_builder.header(key, value);
        }
    }

    // 构建完整请求
    let forward_request = match request_builder.body(Full::new(whole_body)) {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Failed to build forward request: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from("Failed to build forward request")))
                .unwrap());
        }
    };

    // 发送请求并获取响应
    let response = match sender.send_request(forward_request).await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Failed to send request: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(format!(
                    "Failed to send request: {}",
                    e
                ))))
                .unwrap());
        }
    };

    // 在消费响应之前提取状态和版本
    let status = response.status();
    let version = response.version();
    let response_headers = response.headers().clone();

    // 收集响应体（这会消费 response）
    let response_body = match response.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            eprintln!("Failed to collect response body: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from("Failed to read response body")))
                .unwrap());
        }
    };

    // 构建返回响应（使用之前提取的信息）
    let mut response_builder = Response::builder().status(status).version(version);

    // 复制响应头（使用之前提取的 headers）
    for (key, value) in response_headers.iter() {
        response_builder = response_builder.header(key, value);
    }

    match response_builder.body(Full::new(response_body)) {
        Ok(resp) => Ok(resp),
        Err(e) => {
            eprintln!("Failed to build response: {}", e);
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from("Failed to build response")))
                .unwrap())
        }
    }
}

pub async fn handle_https_without_cert(
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let target = req.uri().authority().unwrap().as_str();
    println!("Establishing tunnel to target: {}", target);

    let mut stream = match TcpStream::connect(target).await {
        Ok(_stream) => {
            println!("Connected to target: {}", target);
            _stream
        }
        Err(e) => {
            eprintln!("Failed to connect to target {}: {}", target, e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(format!(
                    "Failed to connect to target {}: {}",
                    target, e
                ))))
                .unwrap());
        }
    };

    //3. 返回给客户端一个Connection Established响应
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(Bytes::from("")))
        .expect("Failed to build response");

    let upgraded = hyper::upgrade::on(req);
    tokio::spawn(async move {
        match upgraded.await {
            Ok(client_stream) => {
                let mut upgraded = TokioIo::new(client_stream);
                let _ = tokio::io::copy_bidirectional(&mut upgraded, &mut stream).await;
            }
            Err(e) => {
                eprintln!("Upgrade error: {}", e);
            }
        }
    });

    Ok(response)
}

pub async fn handle_https_with_cert(
    mut req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let authority = req.uri().authority().unwrap().as_str();
    // 原域名不含端口号时，默认443端口
    let (domain, port) = authority.split_once(':').unwrap_or((authority, "443"));
    println!("建立MITM隧道：{}:{}", domain, port);

    //1. 返回给客户端一个Connection Established响应
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(Bytes::from("")))
        .expect("Failed to build response");

    // 升级获得更底层stream流的读写能力，即TCP层
    let upgraded: hyper::upgrade::OnUpgrade = hyper::upgrade::on(&mut req);

    tokio::spawn(async move {
        match upgraded.await {
            Ok(new_stream) => {
                println!("与客户端完成升级");
                // move的时候没有将domain移动过来吗？？
                let host = req.uri().host().unwrap();
                // 与客户端建立连接后，可以在这里处理流量
                handle_upgraded_https_traffics(new_stream, host).await;
            }
            Err(e) => {
                eprintln!("升级错误: {}", e);
            }
        }
    });
    Ok(response)
}

async fn handle_upgraded_https_traffics(stream: hyper::upgrade::Upgraded, host: &str) {
    // 1. 加载根证书来为目标服务生成服务器证书
    let (server_ca, server_key) = cert::generate_server_cert(host).unwrap();

    // 2. 利用证书和客户端建立连接
    let (certs, pri_key) = cert::load_cert_from_string(server_ca, server_key).unwrap(); // load_cert().unwrap();
    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, pri_key)
        .expect("Failed to create server config");

    // 官方使用文档：https://github.com/rustls/hyper-rustls/blob/main/examples/server.rs
    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));
    let stream = TokioIo::new(stream);
    let client_tls = tls_acceptor.accept(stream).await.unwrap();

    // 4. 新拉起一个进程在其中完成流量传递。
    tokio::spawn(async move {
        // 代理 <-> 真实服务器：基于 hyper 处理 HTTP 明文
        async fn handle_raw_http_request(
            _req: Request<Incoming>,
        ) -> Result<Response<Full<Bytes>>, Infallible> {
            // 解析，然后转成字节转发。
            let original_uri = _req.uri().clone();
            println!("我们查出来的uri是这样子：{}", original_uri);
            let method = _req.method().clone();
            let version = _req.version();
            let headers = _req.headers().clone();

            // 从 Host 头部获取目标主机
            let host = headers
                .get("Host")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("localhost"); // 默认值

            // 重建完整的 URI
            let scheme = "https"; // 因为是 HTTPS 连接
            let full_uri = if original_uri.path().starts_with('/') {
                format!("{}://{}{}", scheme, host, original_uri)
            } else {
                format!("{}://{}/{}", scheme, host, original_uri)
            };
            println!("重建后的完整URI: {}", full_uri);

            // 构建转发请求（使用之前提取的信息）
            let mut request_builder = Request::builder()
                .method(method)
                .uri(full_uri)
                .version(version);
            for (k, v) in headers.iter() {
                request_builder = request_builder.header(k, v);
            }

            let req_body = _req.collect().await.unwrap();
            let req_body_bytes = req_body.to_bytes();
            let forward_req = request_builder.body(Full::new(req_body_bytes)).unwrap();

            // 3. 代替客户端和真实目标建立https连接
            let client_config = ClientConfig::builder()
                .with_native_roots()
                .unwrap()
                .with_no_client_auth();
            let https_connector = HttpsConnectorBuilder::new()
                .with_tls_config(client_config)
                .https_or_http()
                .enable_http1()
                .build();
            let as_client =
                Client::builder(TokioExecutor::new()).build::<_, Full<Bytes>>(https_connector);

            let resp = as_client.request(forward_req).await;
            let resp = match resp {
                Ok(response) => response,
                Err(e) => {
                    eprintln!("Failed to get response from target server: {}", e);
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .body(Full::new(Bytes::from(
                            "Failed to get response from target server",
                        )))
                        .unwrap());
                }
            };
            
            // 增加结果解析和替换
            let status = resp.status();
            let version = resp.version();
            let resp_headers = resp.headers().clone();
            
            let resp_body = match resp.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    eprintln!("Failed to collect response body: {}", e);
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .body(Full::new(Bytes::from("Failed to read response body")))
                        .unwrap());
                }
            };
            
            let mut response_builder = Response::builder().status(status).version(version);
        
            // 复制响应头（使用之前提取的 headers）
            for (key, value) in resp_headers.iter() {
                response_builder = response_builder.header(key, value);
            }

            let response = response_builder.body(Full::new(Bytes::from(resp_body))) // 响应体内容
                .unwrap();
            Ok(response)
        }

        let mut client_io = TokioIo::new(client_tls);
        if let Err(e) = Builder::new()
            .serve_connection(&mut client_io, service_fn(handle_raw_http_request))
            .await
        {
            eprintln!("Error serving connection: {}", e);
        }
    });
}
