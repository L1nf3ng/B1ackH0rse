use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1::Builder;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;

async fn hello_world(
    _req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

pub async fn minimal_hyper_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 开始本地监听8888端口。
    let addr = SocketAddr::from(([127, 0, 0, 1], 8090));
    let listener = TcpListener::bind(&addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            let conn = Builder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(io, service_fn(hello_world));

            if let Err(err) = conn.await {
                eprintln!("Failed to serve connection {:?}", err)
            }
        });
    }
}
