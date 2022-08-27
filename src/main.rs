use tonic::{transport::Server, Request, Response, Status};
pub mod jigsaw {
    tonic::include_proto!("jigsaw");
}
// soon I will actually use the client to call other jigsaw instances
// use jigsaw::jigsaw_client::JigsawClient;
use jigsaw::jigsaw_server::{Jigsaw, JigsawServer};
use jigsaw::Nothing;

pub struct MyJigsaw {}

#[tonic::async_trait]
impl Jigsaw for MyJigsaw {
    async fn a(&self, request: Request<Nothing>) -> Result<Response<Nothing>, Status> {
        println!("Got a request: {:?}", request);
        Ok(Response::new(Nothing {}))
    }
    async fn b(&self, request: Request<Nothing>) -> Result<Response<Nothing>, Status> {
        println!("Got a request: {:?}", request);
        Ok(Response::new(Nothing {}))
    }
    async fn c(&self, request: Request<Nothing>) -> Result<Response<Nothing>, Status> {
        println!("Got a request: {:?}", request);
        Ok(Response::new(Nothing {}))
    }
}

// #[tokio::main]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let addr = "[::1]:50051".parse()?; // apparently ipv6 isn't working, maybe only in docker containers?
    let addr = "127.0.0.1:6379".parse()?;
    let jigsaw = MyJigsaw {};

    Server::builder()
        .add_service(JigsawServer::new(jigsaw))
        .serve(addr)
        .await?;

    Ok(())
}
