use async_recursion::async_recursion;
use tonic::{Request, Response, Status};
mod tonic_thespian {
    tonic::include_proto!("thespian");
}
use opentelemetry::{global::get_text_map_propagator, propagation::Injector};
use rand::{thread_rng, Rng};
use serde::Deserialize;
use tokio::time::{sleep_until, Duration, Instant};
use tonic_thespian::{
    thespian_client::ThespianClient,
    thespian_server::{Thespian, ThespianServer},
    Nothing,
};
use tracing::{error, info, instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::get_trace_id::get_trace_id;

type ThespianResult = Result<(), String>;

struct TonicMetadataMap<'a>(&'a mut tonic::metadata::MetadataMap);

impl<'a> Injector for TonicMetadataMap<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(key) = tonic::metadata::MetadataKey::from_bytes(key.as_bytes()) {
            if let Ok(val) = tonic::metadata::MetadataValue::try_from(&value) {
                self.0.insert(key, val);
            }
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ThespianInstance {
    service_name: String,
    grpc_method_a: OptionalFunction,
    grpc_method_b: OptionalFunction,
    grpc_method_c: OptionalFunction,
}

impl ThespianInstance {
    pub fn new(config_json: &str) -> Self {
        serde_json::from_str(config_json).unwrap()
    }

    // todo: try to make this cleaner
    // other modules (like main) can't easily recognize the same tonic-built types (contained in the "tonic_thespian" module defined at the top of this file)
    // there is a way, but it's intrusive. You have to make the tonic_thespian module public and then use the following two lines in other modules:
    /*
        use crate::tonic_thespian::thespian_server::Thespian;
        use thespian_instance::tonic_thespian;
    */
    // but because the only other module in question really only needs something that implements all the traits that make up a tonic "Service", and that's all neatly wrapped up for us in the generated ThespianServer type, we can just return the ThespianServer type here and be done with it
    // todo: this is dependent on how to expose the service name, but consider making the only public function in this file one that outputs a ThespianServer
    pub fn as_server(self) -> ThespianServer<Self> {
        ThespianServer::new(self)
    }

    pub fn service_name(&self) -> String {
        self.service_name.clone()
    }
}

#[tonic::async_trait]
impl Thespian for ThespianInstance {
    #[instrument(skip_all, fields(trace_id = get_trace_id()))]
    async fn a(&self, _request: Request<Nothing>) -> Result<Response<Nothing>, Status> {
        self.grpc_method_a.enact().await
    }
    #[instrument(skip_all, fields(trace_id = get_trace_id()))]
    async fn b(&self, _request: Request<Nothing>) -> Result<Response<Nothing>, Status> {
        self.grpc_method_b.enact().await
    }
    #[instrument(skip_all, fields(trace_id = get_trace_id()))]
    async fn c(&self, _request: Request<Nothing>) -> Result<Response<Nothing>, Status> {
        self.grpc_method_c.enact().await
    }
}

// I don't want to copy and paste this `if let` logic for each grpc method, hence the following impl
#[derive(Deserialize, Debug)]
struct OptionalFunction(Option<Function>);

impl OptionalFunction {
    async fn enact(&self) -> Result<Response<Nothing>, Status> {
        if let Some(f) = &self.0 {
            if let Err(e) = f.enact().await {
                return Err(Status::internal(e));
            }
        }
        Ok(Response::new(Nothing {}))
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
    async fn enact(&self) -> ThespianResult {
        for operation in &self.operations {
            operation.enact().await?;
        }
        Ok(())
    }
}

#[derive(Deserialize, Debug)]
enum Operation {
    ConcurrentActions(Vec<Action>),
    Action(Action),
}

impl Operation {
    async fn enact(&self) -> ThespianResult {
        match self {
            Operation::ConcurrentActions(actions) => {
                let action_futures: Vec<_> = actions.iter().map(|action| action.enact()).collect();
                // collect() offers a quick but imprecise way to turn a Vec<Result<T, E>> into a Result<Vec<T>, E>. That's good enough for this use case
                futures::future::join_all(action_futures)
                    .await
                    .into_iter()
                    .collect::<ThespianResult>()?;
            }
            Operation::Action(action) => action.enact().await?,
        }
        Ok(())
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
        failure_chance: Option<f64>,
    },
}

#[derive(Deserialize, Debug)]
enum GrpcMethod {
    A,
    B,
    C,
}

impl Action {
    async fn enact(&self) -> ThespianResult {
        match self {
            Action::Function(f) => f.enact().await?,
            Action::CallService {
                service_address,
                service_port,
                grpc_method,
            } => issue_grpc_request(service_address, service_port, grpc_method).await?,
            Action::Sleep {
                tracing_name,
                duration_ms,
                failure_chance,
            } => sleep(tracing_name, duration_ms, failure_chance).await?,
        }

        Ok(())
    }
}

#[instrument(fields(trace_id = get_trace_id()))]
async fn issue_grpc_request(
    service_address: &str,
    service_port: &str,
    grpc_method: &GrpcMethod,
) -> ThespianResult {
    let service_destination = format!("{service_address}:{service_port}");

    info!("starting grpc request to {service_destination}");

    let mut client = ThespianClient::connect(service_destination.clone())
        .await
        .map_err(|err| err.to_string())?;

    let mut request = tonic::Request::new(Nothing {});

    // this mirrors what main.rs does to propagate trace context from inbound requests: here, we're simply propagating trace context to outbound requests instead
    let otel_context = tracing::Span::current().context();
    let mut request_metadata = TonicMetadataMap(request.metadata_mut());
    get_text_map_propagator(|propagator| {
        propagator.inject_context(&otel_context, &mut request_metadata)
    });

    match grpc_method {
        GrpcMethod::A => client.a(request).await,
        GrpcMethod::B => client.b(request).await,
        GrpcMethod::C => client.c(request).await,
    }
    .map_err(|_status| {
        error!("request to {service_destination} failed");
        r"(ノಠ益ಠ)ノ彡┻━┻"
    })?;

    Ok(())
}

#[instrument(fields(trace_id = get_trace_id()))]
async fn sleep(
    _tracing_name: &str,
    duration_ms: &u64,
    failure_chance: &Option<f64>,
) -> ThespianResult {
    info!("starting sleep for action '{_tracing_name}'");

    sleep_until(Instant::now() + Duration::from_millis(*duration_ms)).await;

    if let Some(failure_chance) = failure_chance {
        let determinant = thread_rng().gen::<f64>();
        if determinant < *failure_chance {
            error!("failed due to random chance (failure_chance was {failure_chance} and rolled {determinant})");
            return Err(r"¯\_(ツ)_/¯".into());
        }
    }

    Ok(())
}
