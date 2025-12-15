use std::net::IpAddr;


pub struct Config {
    pub address: String,
    pub ip: IpAddr,
    pub port: u16,
    pub workers: u8
}


impl Default for Config{
    fn default() -> Self {
        let ip_addr = IpAddr::from([127,0,0,1]);
        Self { 
            //tcp层监听，但仅解析https、http、websocke三种协议，其中https需要安装证书并解密。
            address: String::from("localhost"),
            ip: ip_addr, 
            port: 8090, 
            workers: 1 
        }
    }
}


impl Config{
    pub fn new( port: u16, addr: String, workers: u8) -> Self{
        let ip_addr:IpAddr;
        if addr == "localhost".to_string(){
            ip_addr = IpAddr::from([127,0,0,1]);
        }
        else{
            let ip_pieces: Vec<u8> = addr.split(".").map(|x: &str| x.parse::<u8>().unwrap()).collect();
            if ip_pieces.len() !=4 {
                panic!("the address format is not correct!");
            }
            ip_addr = IpAddr::from([ip_pieces[0], ip_pieces[1], ip_pieces[2], ip_pieces[3]]);
        }
        Self{
            address: addr,
            ip: ip_addr,
            port: port,
            workers: workers
        }
    }
}