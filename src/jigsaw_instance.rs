use async_recursion::async_recursion;

use tonic::{Request, Response, Status};
mod tonic_jigsaw {
    tonic::include_proto!("jigsaw");
}
use tonic_jigsaw::jigsaw_client::JigsawClient;
use tonic_jigsaw::jigsaw_server::{Jigsaw, JigsawServer};
use tonic_jigsaw::Nothing;

use serde::Deserialize;
use tokio::time::{sleep_until, Duration, Instant};

pub struct JigsawInstance {
    config: JigsawConfig,
}

impl JigsawInstance {
    pub fn new(config_json: &str) -> Self {
        let config: JigsawConfig = serde_json::from_str(config_json).unwrap();
        Self { config }
    }

    // other modules (like main) can't easily recognize the same tonic-built types (contained in the "tonic_jigsaw" module defined at the top of this file)
    // there is a way, but it's intrusive. You ahve to make the tonic_jigsaw module public and then use the following two lines in other modules:
    /*
        use crate::tonic_jigsaw::jigsaw_server::Jigsaw;
        use jigsaw_instance::tonic_jigsaw;
    */
    // but because the only other module in question really only needs something that implements all the traits that make up a tonic "Service", and that's all neatly wrapped up for us in the generated JigsawServer type, we can just return the JigsawServer type here and be done with it
    // todo: make the only public function in this file one that outputs a JigsawServer
    pub fn as_server(self) -> JigsawServer<Self> {
        JigsawServer::new(self)
    }
}

#[tonic::async_trait]
impl Jigsaw for JigsawInstance {
    async fn a(&self, request: Request<Nothing>) -> Result<Response<Nothing>, Status> {
        println!("Got a request: {:?}", request);
        self.config.grpc_method_a.enact().await;
        Ok(Response::new(Nothing {}))
    }
    async fn b(&self, request: Request<Nothing>) -> Result<Response<Nothing>, Status> {
        println!("Got a request: {:?}", request);
        self.config.grpc_method_b.enact().await;
        Ok(Response::new(Nothing {}))
    }
    async fn c(&self, request: Request<Nothing>) -> Result<Response<Nothing>, Status> {
        println!("Got a request: {:?}", request);
        self.config.grpc_method_c.enact().await;
        Ok(Response::new(Nothing {}))
    }
}

#[derive(Deserialize, Debug)]
struct JigsawConfig {
    // service_name: String,
    grpc_method_a: OptionalFunction,
    grpc_method_b: OptionalFunction,
    grpc_method_c: OptionalFunction,
}

// I don't want to copy and paste this `if let` for each grpc method, hence the following impl
// rust doesn't let us write impl blocks for externally defined types like Option, so I have to wrap it in my own type
#[derive(Deserialize, Debug, Clone)]
struct OptionalFunction(Option<Function>);

impl OptionalFunction {
    async fn enact(&self) {
        if let Some(f) = &self.0 {
            f.enact().await;
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Function {
    tracing_name: String,
    steps: Vec<Operation>,
}

impl Function {
    #[async_recursion]
    async fn enact(&self) {
        println!(
            "Starting function: {}. Time: {}",
            self.tracing_name,
            chrono::Utc::now().to_rfc3339()
        );
        for step in &self.steps {
            step.enact().await;
        }
        println!(
            "Ending function: {}. Time: {}",
            self.tracing_name,
            chrono::Utc::now().to_rfc3339()
        );
    }
}

#[derive(Deserialize, Debug, Clone)]
enum Operation {
    ConcurrentActions(Vec<Action>),
    Action(Action),
}

impl Operation {
    async fn enact(&self) {
        match self {
            Operation::ConcurrentActions(actions) => {
                let handles: Vec<_> = actions
                    .iter()
                    .map(|action| {
                        // this implementation is probably suboptimal, but I can't pass a borrow into a future because its lifetime isn't necessarily long enough for the future's lifetime. What also kind of sucks is that I have to add the Clone derivation to almost every type in this module
                        let cloned_action = action.clone();
                        tokio::spawn(async move { cloned_action.enact().await })
                    })
                    .collect();
                for handle in handles {
                    handle.await.unwrap();
                }
            }
            Operation::Action(action) => action.enact().await,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
enum Action {
    Function(Function),
    CallService {
        service_address: String,
        service_port: String,
        grpc_method: GrpcMethod,
        // todo
        // timeout_ms: u64,
        // retries: u64,
    },
    Sleep {
        tracing_name: String,
        duration_ms: u64,
    },
}

impl Action {
    async fn enact(&self) {
        match self {
            Action::Function(f) => f.enact().await,
            Action::CallService {
                service_address,
                service_port,
                grpc_method,
            } => {
                // let mut client = JigsawClient::connect("http://localhost:6379")
                let mut client = JigsawClient::connect(format!("{service_address}:{service_port}"))
                    .await
                    .unwrap();

                let request = tonic::Request::new(Nothing {});

                match grpc_method {
                    GrpcMethod::A => client.a(request).await.unwrap(),
                    GrpcMethod::B => client.b(request).await.unwrap(),
                    GrpcMethod::C => client.c(request).await.unwrap(),
                };
            }
            Action::Sleep {
                tracing_name,
                duration_ms,
            } => {
                println!(
                    "Starting action: {}. Time: {}",
                    tracing_name,
                    chrono::Utc::now().to_rfc3339()
                );
                sleep_until(Instant::now() + Duration::from_millis(*duration_ms)).await;
                println!(
                    "Ending action: {}. Time: {}",
                    tracing_name,
                    chrono::Utc::now().to_rfc3339()
                );
            }
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
enum GrpcMethod {
    A,
    B,
    C,
}