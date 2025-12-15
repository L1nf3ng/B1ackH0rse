use hyper::{Body, Request, Response};
use hyper::service::{make_service_fn, service_fn};
use hyper::server::Server;  // 说明：rust的包默认采取最少引入的方式，而在hyper中Server放在cfg_feature!宏下，所以它不会被默认引入。
use std::convert::Infallible;
use std::net::{SocketAddr};

async fn hello_world(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from("Hello, World!")))
}

pub async fn minimal_hyper_server() {
    // 开始本地监听8888端口。
    let addr = SocketAddr::from(([127,0,0,1], 8888));
    // 创建服务工厂，里面的函数决定了处理请求时的行为。    
    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(hello_world))
    });

    // 生成server并引入工厂类
    let server = Server::bind(&addr).serve(make_svc);

    // 运行server
    println!("Listening on http://{}", addr);
    if let Err(e) = server.await {
        eprintln!("服务器错误: {}", e);
    }
}

