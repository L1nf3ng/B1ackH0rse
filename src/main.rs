use B1ackH0rse::network_engine::server::Server;
use std::error::Error;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>{
    let server = Server::default();
    server.start().await?;
    println!("Received NULL character, so we ended server!");
    Ok(())
}
