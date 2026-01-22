use http_body_util::{BodyExt, Empty, Full};
use hyper::body::{Bytes, Incoming};
use hyper::client::conn::http1;
use hyper::{Method, Request, Response, StatusCode};

use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::net::TcpStream;

// hyper升级的官方使用说明：https://hyper.rs/guides/1/upgrading/

pub async fn proxy_services(_req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    match _req.method() {
        &Method::CONNECT => {
            println!("Received HTTPS request from ...");
            // Ok(Response::builder()
            //     .status(StatusCode::OK)
            //     .body(Full::new(Bytes::from("")))
            //     .unwrap())
            // Step1. 先用解密https的方式尝试连接，
            // Step2. 失败后再用纯代理的方式转发。
            handle_https_without_cert(_req).await
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
    req: Request<Incoming>,
    _remote: SocketAddr,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let authority = req.uri().authority().unwrap().as_str();
    let (domain, port) = authority.split_once(':').unwrap_or((authority, "443"));
    println!("建立MITM隧道：{}:{}", domain, port);

    Ok(Response::new(Full::new(Bytes::from("MITM proxy handling"))))
}
