use hyper::{Body, Client, Method, Request, Response};
use std::net::SocketAddr;


pub async fn proxy_services(_req: Request<Body>, remote: SocketAddr ) -> Result<Response<Body>,  hyper::Error> {
    //初始化一个http客户端用来做转发用
    let client= hyper::Client::new();

    match _req.method() {
        &Method::CONNECT => {
            // 对于HTTPS来说开启Connect隧道，制作字节转发
            println!("Received HTTPS CONNECT request from {}", remote);
                // 返回响应
            Ok(Response::new(Body::from("Hello from B1ackH0rse proxy server!")))
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
