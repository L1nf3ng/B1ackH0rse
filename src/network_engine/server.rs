use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use std::error::Error;

pub struct Server{
    address: String,
    port: u32,
    workers: u8
}

impl Default for Server{
    fn default() -> Self {
        Server { 
            //tcp层监听，但仅解析https、http、websocke三种协议，其中https需要安装证书并解密。
            address: "localhost".to_string(), 
            port: 8090, 
            workers: 1 
        }
    }

}


impl Server{
    pub fn new( port: u32, addr: String, workers: u8) -> Self{
        Self{
            address: addr,
            port: port,
            workers: workers
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn Error>>{
        let bind_str = format!("{}:{}", self.address, self.port);
        let server = TcpListener::bind(bind_str).await?;
        
        loop {
            let mut res: String = "".to_string();
            let (mut socket, cli_addr) = server.accept().await?;
            loop{
                let mut buffer = [0;4098];      // 4KB unit
                if let Ok(n) = socket.read(&mut buffer).await {
                    if n == 0 {
                        break
                    }
                    println!("The request comes from {cli_addr}");
                    let message = String::from_utf8_lossy(&buffer[..n]);
                    res.push_str(&message);
                }
                else{
                    eprintln!("we received some wrong data!");
                    break;
                }
            }
            println!("we received following data: {res}");
            let response:String = format!("200\r\n\r\nConnection: Close\r\n\r\n我们已收到你的长度为{}的请求",res.len());
            socket.write_all(response.as_bytes()).await;
            break;
        }
        Ok(())
    }
}

