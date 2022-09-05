use opentelemetry::sdk::{
    trace,
    trace::IdGenerator,
    // trace::{self, IdGenerator, Sampler},
    Resource,
};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use std::env;
use tonic::transport::Server;
use tracing_subscriber::layer::SubscriberExt;
// use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{
    //prelude::*,
    EnvFilter,
    Registry,
};
mod jigsaw_instance;
use jigsaw_instance::JigsawInstance;
use url::Url;

mod get_trace_id;

// todo: I think using "current_thread" instead of the fuller version is noticeably slowing it down. My rudimentary tracing indicates seconds of delay
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // First, create a OTLP exporter builder. Configure it as you need.
    let tempo_otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        // I may have to use this because tempo expects otlp-style interactions on this port?
        .with_endpoint("http://localhost:4317");
    // // I'll try this endpoint because tempo is logging that it's listening on this port
    // .with_endpoint("http://localhost:9095");
    // // this is the normal tempo port, but... maybe it's not used for this?
    // .with_endpoint("http://localhost:3200");

    // // I guess I'm trying http now
    // .http() // have to use the "http-proto" feature
    // .with_endpoint("http://localhost:3200");

    // Then pass it into pipeline builder
    let tempo_otlp_tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(tempo_otlp_exporter)
        .with_trace_config(
            trace::config()
                // .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(IdGenerator::default()) // this is unnecessary to specify, but I include it here as a reminder that apparently this tracer is what actually chooses the trace ids as they show up everywhere else
                // .with_max_events_per_span(64)
                // .with_max_attributes_per_span(16)
                // .with_max_events_per_span(16)
                .with_resource(Resource::new(vec![KeyValue::new("app_name", "example")])),
        )
        .install_simple()?;

    // Finish layers

    let tempo_otlp_layer = tracing_opentelemetry::layer().with_tracer(tempo_otlp_tracer);

    let stdout_log_layer = tracing_subscriber::fmt::layer().pretty();

    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let (loki_layer, loki_layer_task) = tracing_loki::layer(
        // Url::parse("http://127.0.0.1:3100").unwrap(),
        Url::parse("http://localhost:3100").unwrap(),
        vec![("app_name".into(), "example".into())]
            .into_iter()
            .collect(),
        vec![].into_iter().collect(),
    )?;
    // this appears to be analogous to the "install" step for the otlp exporter. It can use a simple exporter that performs an export immediately whenever relevant, or it can use a batch exporter to do it in the background, which I assume is what this "task" is doing
    tokio::spawn(loki_layer_task);

    // Register all subscribers
    let collector = Registry::default()
        .with(loki_layer)
        .with(tempo_otlp_layer)
        .with(stdout_log_layer)
        .with(filter_layer);

    tracing::subscriber::set_global_default(collector).unwrap();

    // and now for the app code

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
