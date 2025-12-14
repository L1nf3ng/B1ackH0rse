use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use std::collections::HashMap;
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
            let mut res: String = String::new();
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
            println!("we received following data:\n {res}");
            // 在这里处理请求，对的。
            // 在这里完成目标地址改写，也就是代理作为中继，需要按照Host值做请求的重新发送
            if res.contains("HTTP") {
                self.handle_http_requests(res).await?;
                self.forward_request().await?;
                self.forward_response().await?;

            }


            let response:String = format!("200\r\n\r\nConnection: Close\r\n\r\n我们已收到你的长度为的请求");
            socket.write_all(response.as_bytes()).await;
            break;
        }
        Ok(())
    }


    pub async fn handle_http_requests(&self, request_str: String) -> Result<(), Box<dyn Error>> {
        let mut request_lines = request_str.lines();

        if let Some(first_line) = request_lines.next(){
            let first_row:Vec<&str> = first_line.split_whitespace().collect();
            if first_row.len()<3 {
                panic!("the http request has a bad format!");
            }
            let method = first_row[0];
            let uri = first_row[1];
            let protocol = first_row[2];
            println!("the request protocal is {protocol}; method {method}; uri {uri}");
            let mut header_map: HashMap<&str, &str> = HashMap::new();
            for line in request_lines{
                if let Some(pos_idx) = line.find(":"){
                    let key = line[..pos_idx].trim();
                    let value = line[pos_idx+1..].trim();
                    header_map.insert(key, value);
                }
            }
            println!("the requset headers are: {:?}", header_map);

        }

        Ok(())
    }


    pub async fn forward_request(&self) -> Result<(), &'static str>{
        ///这个函数将source的请求发送给target
        todo!();
    }


    pub async fn forward_response(&self) -> Result<(), &'static str>{
        ///这个函数将收到的server端响应转发给source
        todo!();
    }
}

