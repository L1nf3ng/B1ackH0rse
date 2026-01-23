use B1ackH0rse::config::Config;
use B1ackH0rse::network_engine::server::proxy_services;
use B1ackH0rse::utils::cert;
use hyper::server::conn::http1::Builder;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
// use std::error::Error;
use std::net::SocketAddr;
use std::path::Path;
use std::fs;
use std::env;


fn print_usage(){
    let logo = r#"
        __      __               __         
    \ \    / /__  _ __ ___  / /__  ___  
     \ \/\/ / _ \| '__/ _ \/ / _ \/ __| 
      \  / / (_) | | |  __/ /  __/\__ \ 
       \/  \___/|_|  \___/_/\___||___/ 
    ======================================
    Whorse - Hello from the proxy
        "#;
    let usage = r#"
    NAME:
        Whorse - A powerful passive scanner engine [https://github.com/L1nf3ng/Whorse]
    
    USAGE:
        [global options] command [command options] [arguments...]
        
    COMMANDS:
        webscan, ws        Run a webscan task, in a passive mode.
        genca              GenerateToFile CA certificate and key
        list               A command that show all enabled plugins.
        help, h            Shows a list of commands or help for one command
    You can customize new commands or modify the plugins enabled by a command in the configuration file.
    GLOBAL OPTIONS:
        --config FILE      Load configuration from FILE (default: "config.yaml")
        --log-level value  Log level, choices are debug, info, warn, error, fatal
        "#;
    println!("{}", logo);
    println!("{}", usage);
}


#[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {
async fn main(){
    // 初始化日志
    env_logger::init();
    
    // 增加命令行参数处理逻辑。
    let mut args = env::args().skip(1); // 不分析程序名字本身
    if args.len() == 0 {
        print_usage();
        println!("请按照以上格式传入正确的参数！！！");
        std::process::exit(-1);
    }
    while let Some(arg) = args.next() {
        // 转换成str切片
        match arg.as_str() {
            "genca"=>{
                let output_dir = "./output/";
                match cert::generate_ca_cert() {
                    Ok((cert_der, key_der)) => {
                        // 增加路径不存在则新建的能力。
                        if Path::new(output_dir).exists() == false {
                            fs::create_dir(output_dir).unwrap();
                        }
                        let cert_path = output_dir.to_string() + "cert.pem";
                        let key_path = output_dir.to_string() + "prikey.pem";

                        if Path::new(&cert_path).exists() {
                            println!("Certificate file already exists: {}", cert_path);
                        } else {
                            fs::write(&cert_path, cert_der).unwrap();
                            println!("Certificate saved to: {}", cert_path);
                        }

                        if Path::new(&key_path).exists() {
                            println!("Private key file already exists: {}", key_path);
                        } else {
                            fs::write(&key_path, key_der).unwrap();
                            println!("Private key saved to: {}", key_path);
                        }
                    }
                    Err(e) => {
                        panic!("Failed to generate certificate: {}", e);
                    }
                }
            },
            "webscan" | "ws"=>{
                // todo!增加从命令行参数或配置文件读配置的逻辑。
                let config: Config = Config::default();
                // 这里我们切换成hyper server
                println!("Starting server at {}:{}", config.address, config.port);
                let addr = SocketAddr::from((config.ip, config.port));
                let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            
                loop {
                    let (stream, _) = listener.accept().await.unwrap();
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
            },
            "--help"| "-h" =>{
                print_usage();
                std::process::exit(0);
            },
            _ => {
                print_usage();
                std::process::exit(-1);
            }
        }
    }
}
