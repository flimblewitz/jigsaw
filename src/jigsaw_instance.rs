use async_recursion::async_recursion;
use tonic::{Request, Response, Status};
mod tonic_jigsaw {
    tonic::include_proto!("jigsaw");
}
use opentelemetry::global;
use opentelemetry::propagation::Injector;
use serde::Deserialize;
use tokio::time::{sleep_until, Duration, Instant};
use tonic_jigsaw::jigsaw_client::JigsawClient;
use tonic_jigsaw::jigsaw_server::{Jigsaw, JigsawServer};
use tonic_jigsaw::Nothing;
use tracing::{info, instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::get_trace_id::get_trace_id;

struct MetadataMap<'a>(&'a mut tonic::metadata::MetadataMap);

impl<'a> Injector for MetadataMap<'a> {
    /// Set a key and value in the MetadataMap.  Does nothing if the key or value are not valid inputs
    fn set(&mut self, key: &str, value: String) {
        if let Ok(key) = tonic::metadata::MetadataKey::from_bytes(key.as_bytes()) {
            if let Ok(val) = tonic::metadata::MetadataValue::from_str(&value) {
                self.0.insert(key, val);
            }
        }
    }
}

pub struct JigsawInstance {
    config: JigsawConfig,
}

impl JigsawInstance {
    pub fn new(config_json: &str) -> Self {
        let config: JigsawConfig = serde_json::from_str(config_json).unwrap();
        Self { config }
    }

    // todo: try to make this cleaner
    // other modules (like main) can't easily recognize the same tonic-built types (contained in the "tonic_jigsaw" module defined at the top of this file)
    // there is a way, but it's intrusive. You have to make the tonic_jigsaw module public and then use the following two lines in other modules:
    /*
        use crate::tonic_jigsaw::jigsaw_server::Jigsaw;
        use jigsaw_instance::tonic_jigsaw;
    */
    // but because the only other module in question really only needs something that implements all the traits that make up a tonic "Service", and that's all neatly wrapped up for us in the generated JigsawServer type, we can just return the JigsawServer type here and be done with it
    // todo: this is dependent on how to expose the service name, but consider making the only public function in this file one that outputs a JigsawServer
    pub fn as_server(self) -> JigsawServer<Self> {
        JigsawServer::new(self)
    }

    // todo: try to make this cleaner
    pub fn get_service_name(&self) -> String {
        self.config.service_name.clone()
    }
}

#[tonic::async_trait]
impl Jigsaw for JigsawInstance {
    #[instrument(skip_all)]
    async fn a(&self, _request: Request<Nothing>) -> Result<Response<Nothing>, Status> {
        self.config.grpc_method_a.enact().await;
        Ok(Response::new(Nothing {}))
    }
    #[instrument(skip_all)]
    async fn b(&self, _request: Request<Nothing>) -> Result<Response<Nothing>, Status> {
        self.config.grpc_method_b.enact().await;
        Ok(Response::new(Nothing {}))
    }
    #[instrument(skip_all)]
    async fn c(&self, _request: Request<Nothing>) -> Result<Response<Nothing>, Status> {
        self.config.grpc_method_c.enact().await;
        Ok(Response::new(Nothing {}))
    }
}

#[derive(Deserialize, Debug)]
struct JigsawConfig {
    service_name: String,
    grpc_method_a: OptionalFunction,
    grpc_method_b: OptionalFunction,
    grpc_method_c: OptionalFunction,
}

// I don't want to copy and paste this `if let` for each grpc method, hence the following impl
// rust doesn't let us write impl blocks for externally defined types like Option, so I have to wrap it in my own type
#[derive(Deserialize, Debug)]
struct OptionalFunction(Option<Function>);

impl OptionalFunction {
    async fn enact(&self) {
        if let Some(f) = &self.0 {
            f.enact().await;
        }
    }
}

#[derive(Deserialize, Debug)]
struct Function {
    tracing_name: String,
    operations: Vec<Operation>,
}

impl Function {
    #[async_recursion]
    #[instrument(name = "Function.enact", skip(self), fields(trace_id = get_trace_id(), _tracing_name = self.tracing_name))]
    async fn enact(&self) {
        info!("starting");
        for operation in &self.operations {
            operation.enact().await;
        }
        info!("ending");
    }
}

#[derive(Deserialize, Debug)]
enum Operation {
    ConcurrentActions(Vec<Action>),
    Action(Action),
}

impl Operation {
    async fn enact(&self) {
        match self {
            Operation::ConcurrentActions(actions) => {
                let action_futures: Vec<_> = actions.iter().map(|action| action.enact()).collect();
                futures::future::join_all(action_futures).await;
            }
            Operation::Action(action) => action.enact().await,
        }
    }
}

#[derive(Deserialize, Debug)]
enum Action {
    Function(Function),
    CallService {
        service_address: String,
        service_port: String,
        grpc_method: GrpcMethod,
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
            } => issue_grpc_request(service_address, service_port, grpc_method).await,
            Action::Sleep {
                tracing_name,
                duration_ms,
            } => sleep(tracing_name, duration_ms).await,
        }
    }
}

// this is a triumphant confluence of the tonic, opentelemetry, tracing, and tracing_opentelemetry crates
// it mirrors what main.rs is doing, except now we're propagating trace context via an outbound request instead of capturing it from an inbound request
// tracing_opentelemetry gives us an extension trait with a tracing::Span::context method that creates an opentelemetry::Context from a tracing::Span
// opentelemetry gives us the global::get_text_map_propagator function, which is what actually what facilitates the propagation of the trace context via the outbound request
#[instrument(fields(trace_id = get_trace_id()))]
async fn issue_grpc_request(service_address: &str, service_port: &str, grpc_method: &GrpcMethod) {
    info!("starting");

    let mut client = JigsawClient::connect(format!("{service_address}:{service_port}"))
        .await
        .unwrap();

    let mut request = tonic::Request::new(Nothing {});

    let otel_context = tracing::Span::current().context();

    let mut request_metadata = MetadataMap(request.metadata_mut());

    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&otel_context, &mut request_metadata)
    });

    match grpc_method {
        GrpcMethod::A => client.a(request).await.unwrap(),
        GrpcMethod::B => client.b(request).await.unwrap(),
        GrpcMethod::C => client.c(request).await.unwrap(),
    };

    info!("ending");
}

#[instrument(fields(trace_id = get_trace_id()))]
async fn sleep(_tracing_name: &str, duration_ms: &u64) {
    info!("starting");
    sleep_until(Instant::now() + Duration::from_millis(*duration_ms)).await;
    info!("ending");
}

#[derive(Deserialize, Debug)]
enum GrpcMethod {
    A,
    B,
    C,
}
