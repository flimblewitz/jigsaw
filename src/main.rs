use std::env;
use tonic::transport::Server;
mod jigsaw_instance;
use jigsaw_instance::JigsawInstance;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // todo: replace this hardcoded config with an environment variable
    let config_json = env::var("CONFIG_JSON")?;
    let jigsaw_server = JigsawInstance::new(&config_json).as_server();

    // let addr = "[::1]:50051".parse()?; // apparently ipv6 isn't working, maybe only in docker containers?
    let addr = "127.0.0.1:6379".parse()?;

    Server::builder()
        .add_service(jigsaw_server)
        .serve(addr)
        .await?;

    Ok(())
}
