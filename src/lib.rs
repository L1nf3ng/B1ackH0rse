pub mod interface;
pub mod rules_engine;
pub mod network_engine;
pub mod utils;
pub mod config;


#[cfg(test)]
mod tests{

    use crate::utils::cert;
    use super::network_engine::learn_hyper::minimal_hyper_server;
    use super::utils::cert::generate_cert;
    use core::panic;
    use std::fs;
    use std::path::Path;

    // #[tokio::test]
    // async fn test_minimal_server(){
    //     assert_eq!(minimal_hyper_server().await, ());
    // }

    #[tokio::test]
    async fn test_certifactes(){
        let output_dir = "./output/";
        match generate_cert() {
            Ok((cert_der, key_der)) => {
                // 增加路径不存在则新建的能力。
                if Path::new(output_dir).exists() == false {
                    fs::create_dir(output_dir).unwrap();
                }

                fs::write(output_dir.to_string() + "cert.pem", cert_der).unwrap();
                fs::write(output_dir.to_string() + "prikey.pem", key_der).unwrap();
            },
            Err(e) => {
                panic!("Failed to generate certificate: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_load_certifactes(){
        match cert::load_cert() {
            Ok((certs, key)) => {
                assert!(certs.len() > 0);
            },
            Err(e) => {
                panic!("Failed to load certificate: {}", e);
            }
        }
    }

}
