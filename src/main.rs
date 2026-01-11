use B1ackH0rse::config::Config;
use B1ackH0rse::network_engine::server::proxy_services;
use hyper::server::conn::http1::Builder;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::error::Error;
use std::net::{SocketAddr, TcpListener};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 初始化日志
    env_logger::init();

    // todo!增加从命令行参数或配置文件读配置的逻辑。
    let config: Config = Config::default();
    // 这里我们切换成hyper server
    println!("Starting server at {}:{}", config.address, config.port);
    let addr = SocketAddr::from((config.ip, config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = Builder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(io, service_fn(proxy_services))
                .with_upgrades()
                .await
            {
                eprintln!("Failed to serve connection {:?}", err)
            }
        });
    }
}
