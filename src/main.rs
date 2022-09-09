use http;
use opentelemetry::{
    global,
    propagation::{Extractor, TextMapPropagator},
    sdk::{propagation::TraceContextPropagator, trace, trace::IdGenerator, Resource},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use std::env;
use tonic::transport::Server;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use url::Url;

// rust requires that we explicitly define module hierarchy in code, and main.rs defines the crate (root) module. It's analogous to a hypothetical foo.rs module file and its optional sibling foo/ folder containing submodules
mod get_trace_id;
mod jigsaw_instance;
use jigsaw_instance::JigsawInstance;

struct HeaderMap<'a>(&'a http::HeaderMap);

impl<'a> Extractor for HeaderMap<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }
    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|key| key.as_str()).collect::<Vec<_>>()
    }
}

// the current_thread flavor of tokio is being used because it doesn't seem to matter for something as lightweight and hollow as jigsaw
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_json = env::var("CONFIG_JSON")?;
    let jigsaw_instance = JigsawInstance::new(&config_json);

    install_tracing(jigsaw_instance.get_service_name());

    let port = env::var("PORT").unwrap_or("6379".into());
    let addr = format!("127.0.0.1:{port}").parse()?;

    Server::builder()
        .trace_fn(|request| {
            // this is a triumphant confluence of the tonic, opentelemetry, tracing, and tracing_opentelemetry crates
            // tonic gives us this the trace_fn() method to initiate a span in a custom way for each inbound request
            // opentelemetry gives us the trace context propagation tools for preserving trace context across requests (for both inbound and outbound requests), though we do have to do some legwork of our own. In this case, that means implementing opentelemetry::propagation::Extractor for the request headers since they are the medium by which trace context is propagated. The official example for tonic actually draws the trace context from individual tonic requests' metadata (which is simply derived from the headers), but that's not as conveniently generic as this implementation since it has to be duplicated for each individual type of tonic request that you want to instrument
            // tracing gives us a way to create spans that we can easily log and export
            // tracing_opentelemetry gives us an extension trait with a tracing::Span::set_parent method that sets a tracing::Span's parent to an opentelemetry::Context, allowing us to preserve the inbound request's trace context in all forthcoming spans
            // and though it's not obvious, if there's no trace context in the inbound request, we'll automatically get a new random trace id for a new root span instead

            let carrier = HeaderMap(request.headers());

            let propagator = TraceContextPropagator::new();

            let parent_context = propagator.extract(&carrier);

            let span = tracing::span!(tracing::Level::INFO, "request received");

            span.set_parent(parent_context.clone());

            span
        })
        .add_service(jigsaw_instance.as_server())
        .serve(addr)
        .await?;

    Ok(())
}

fn install_tracing(service_name: String) {
    // this enables us to later access the opentelemetry global text map propagator whenever we want to make an outgoing request to another microservice and propagate trace context to them
    global::set_text_map_propagator(TraceContextPropagator::new());

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
