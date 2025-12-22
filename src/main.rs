use B1ackH0rse::config::Config;
use B1ackH0rse::network_engine::server::proxy_services;
use hyper::server::{Server, conn::AddrStream};
use hyper::service::{make_service_fn, service_fn};
use std::error::Error;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 初始化日志
    env_logger::init();

    // todo!增加从命令行参数或配置文件读配置的逻辑。
    let config: Config = Config::default();
    // 这里我们切换成hyper server
    println!("Starting server at {}:{}", config.address, config.port);
    let addr = SocketAddr::from((config.ip, config.port));
    let make_mvc = make_service_fn(move |_conn: &AddrStream| {
        let remote = _conn.remote_addr();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                // 在这个异步函数里写处理逻辑，成功时返回Response Body内容
                proxy_services(req, remote)
            }))
        }
    });
    let server = Server::bind(&addr).serve(make_mvc);
    // 优雅关闭
    if let Err(e) = server.await {
        eprintln!("服务器异常退出: {}", e);
    }
    Ok(())
}
