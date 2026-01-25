use std::net::IpAddr;
use std::fs::{File, create_dir};
use std::io::Write;
use std::path::Path;
use env_logger::{Builder, Target};
use time::{OffsetDateTime, macros::format_description};


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

pub fn setup_file_logger(){
    
    let log_dir = "logs";
    // 增加日志路径存在与否判断
    if Path::new(log_dir).exists() == false {
        create_dir(log_dir).unwrap();
    }
    
    let log_file = File::create("logs/app.log").expect("You didn't create the log file!");
    
    Builder::new().
        format(|buf, record| {
            let now = OffsetDateTime::now_utc();
            let ts_format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]");
            writeln!(
                buf, 
                "[{}] [{}] {}:{} - {}",
                now.format(&ts_format).unwrap(),
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        }).
        target(Target::Pipe(Box::new(log_file))).
        filter(None, log::LevelFilter::Info).
        init();   
}
