use std::env;
use tonic::transport::Server;
mod jigsaw_instance;
use jigsaw_instance::JigsawInstance;

// todo: I think using "current_thread" instead of the fuller version is noticeably slowing it down. My rudimentary tracing indicates seconds of delay
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_json = env::var("CONFIG_JSON")?;
    let jigsaw_server = JigsawInstance::new(&config_json).as_server();

    let port = env::var("PORT").unwrap_or("6379".into());
    // let addr = "[::1]:50051".parse()?; // apparently ipv6 isn't working, maybe only in docker containers?
    let addr = format!("127.0.0.1:{port}").parse()?;

    Server::builder()
        .add_service(jigsaw_server)
        .serve(addr)
        .await?;

    Ok(())
}
