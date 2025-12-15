pub mod interface;
pub mod rules_engine;
pub mod network_engine;
pub mod utils;
pub mod config;



#[cfg(test)]
mod tests{

    use super::network_engine::learn_hyper::minimal_hyper_server;

    #[tokio::test]
    async fn test_minimal_server(){
        assert_eq!(minimal_hyper_server().await, ());
    }
}