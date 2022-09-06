use opentelemetry::sdk::{trace, trace::IdGenerator, Resource};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use std::env;
use tonic::transport::Server;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};
use url::Url;

// rust requires that we explicitly define module hierarchy in code, and main.rs defines the crate (root) module. It's analogous to a hypothetical foo.rs module file and its optional sibling foo/ folder containing submodules
mod get_trace_id;
mod jigsaw_instance;
use jigsaw_instance::JigsawInstance;

// the current_thread flavor of tokio is being used because it doesn't seem to matter for something as lightweight and hollow as jigsaw
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_json = env::var("CONFIG_JSON")?;
    let jigsaw_instance = JigsawInstance::new(&config_json);

    install_tracing(jigsaw_instance.get_service_name());

    let port = env::var("PORT").unwrap_or("6379".into());
    let addr = format!("127.0.0.1:{port}").parse()?;

    Server::builder()
        .add_service(jigsaw_instance.as_server())
        .serve(addr)
        .await?;

    Ok(())
}

fn install_tracing(service_name: String) {
    let tempo_otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        // I have to use this because tempo expects otlp-style interactions on this port
        .with_endpoint("http://localhost:4317");

    let tempo_otlp_tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(tempo_otlp_exporter)
        .with_trace_config(
            trace::config()
                .with_id_generator(IdGenerator::default()) // this is unnecessary to specify, but I include it here as a reminder that apparently this tracer is what actually chooses the trace ids as they show up everywhere else
                .with_resource(Resource::new(vec![KeyValue::new(
                    "service.name",
                    service_name.clone(),
                )])),
        )
        .install_simple()
        .unwrap();

    let tempo_otlp_layer = tracing_opentelemetry::layer().with_tracer(tempo_otlp_tracer);

    let stdout_log_layer = tracing_subscriber::fmt::layer().pretty();

    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    // note: this crate is automatically adding "level" as a label, which is undesired and fixed in an unreleased version
    let (loki_layer, loki_layer_task) = tracing_loki::layer(
        Url::parse("http://localhost:3100").unwrap(),
        vec![("service_name".into(), service_name)]
            .into_iter()
            .collect(),
        vec![].into_iter().collect(),
    )
    .unwrap();
    // this appears to be analogous to the "install" step for the otlp exporter. It can use a simple exporter that performs an export immediately whenever relevant, or it can use a batch exporter to do it in the background, which I assume is what this "task" is doing
    tokio::spawn(loki_layer_task);

    let collector = Registry::default()
        .with(loki_layer)
        .with(tempo_otlp_layer)
        .with(stdout_log_layer)
        .with(filter_layer);

    tracing::subscriber::set_global_default(collector).unwrap();
}
