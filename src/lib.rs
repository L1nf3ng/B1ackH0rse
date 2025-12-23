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
    use std::fs;

    #[tokio::test]
    async fn test_minimal_server(){
        assert_eq!(minimal_hyper_server().await, ());
    }

    #[tokio::test]
    async fn test_certifactes(){
        match generate_cert() {
            Ok((cert_str, key_str)) => {
                fs::write("cert.pem", cert_str).unwrap();
                fs::write("prikey.pem", key_str).unwrap();
            },
            _ => {}
        }

    }
}
