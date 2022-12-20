use http;
use opentelemetry::{
    global::set_text_map_propagator,
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
mod thespian_instance;
use thespian_instance::ThespianInstance;

struct HeaderMap<'a>(&'a http::HeaderMap);

impl<'a> Extractor for HeaderMap<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }
    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|key| key.as_str()).collect::<Vec<_>>()
    }
}

// the current_thread flavor of tokio is being used because it doesn't seem to matter for something as lightweight and hollow as thespian
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_json = env::var("CONFIG_JSON")?;
    let thespian_instance = ThespianInstance::new(&config_json);

    install_tracing(thespian_instance.service_name());

    let port = env::var("PORT").unwrap_or("6379".into());
    let addr = format!("0.0.0.0:{port}").parse()?;

    Server::builder()
        // tonic gives us this the trace_fn() method to initiate a span in a custom way for each inbound request. We'll use it to facilitate trace context propagation
        .trace_fn(|request| {
            // opentelemetry requires us to do some legwork of our own to perform trace context propagation
            // in this case, that means implementing opentelemetry::propagation::Extractor for the request headers since they are the medium by which trace context is propagated
            // opentelemetry's official example for tonic actually extracts the trace context from individual tonic requests' metadata (which is simply derived from the headers), but that's not as conveniently generic as this implementation since it has to be duplicated for each individual type of tonic request that you want to instrument

            let carrier = HeaderMap(request.headers());

            let propagator = TraceContextPropagator::new();

            let parent_context = propagator.extract(&carrier);

            let span = tracing::span!(tracing::Level::INFO, "request received");

            // though it's not obvious, if there's no trace context in the inbound request, we'll end up automatically getting a new random trace id so that this span can be a new root span
            span.set_parent(parent_context.clone());

            span
        })
        .add_service(thespian_instance.as_server())
        .serve(addr)
        .await?;

    Ok(())
}

fn install_tracing(service_name: String) {
    // opentelemetry requires us to do this in order to later be able to propagate trace context via outbound requests
    set_text_map_propagator(TraceContextPropagator::new());

    let stdout_log_layer = tracing_subscriber::fmt::layer().pretty();

    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let collector = Registry::default()
        .with(stdout_log_layer)
        .with(filter_layer);

    let otel_layer = env::var("OTEL_BACKEND_ADDRESS")
        .ok()
        .and_then(|otel_backend_address| {
            if otel_backend_address.is_empty() {
                return None;
            }

            let otlp_exporter = opentelemetry_otlp::new_exporter()
                .tonic()
                // I have to use this because tempo expects otlp-style interactions on this port
                .with_endpoint(otel_backend_address);

            let otlp_tracer = opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(otlp_exporter)
                .with_trace_config(
                    trace::config()
                        // this use of with_id_generator is unnecessary to specify because it's default behavior, but I include it here as a reminder that apparently this tracer is what actually generates new span ids
                        .with_id_generator(IdGenerator::default())
                        .with_resource(Resource::new(vec![KeyValue::new(
                            "service.name",
                            service_name.clone(),
                        )])),
                )
                // although install_simple gets the job done, "real" APIs ought to use install_batch instead for better performance
                .install_simple()
                .unwrap();

            Some(tracing_opentelemetry::layer().with_tracer(otlp_tracer))
        });

    let loki_layer = env::var("LOKI_ADDRESS").ok().and_then(|loki_address| {
        if loki_address.is_empty() {
            return None;
        }

        let (loki_layer, loki_layer_task) = tracing_loki::layer(
            Url::parse(&loki_address).unwrap(),
            vec![("service_name".into(), service_name)]
                .into_iter()
                .collect(),
            vec![].into_iter().collect(),
        )
        .unwrap();

        // this appears to be analogous to the "install" step for tempo_otlp_tracer
        tokio::spawn(loki_layer_task);

        Some(loki_layer)
    });

    // todo: identify a way to conditionally apply layers without having to use a match statement like this. Not being able to rely on the inferred types for the layers and collector makes it hard to factor that code out and allow the collector to be one of several possible types
    match (otel_layer, loki_layer) {
        (None, None) => {
            tracing::subscriber::set_global_default(collector).unwrap();
        }
        (None, Some(loki_layer)) => {
            tracing::subscriber::set_global_default(collector.with(loki_layer)).unwrap();
        }
        (Some(otel_layer), None) => {
            tracing::subscriber::set_global_default(collector.with(otel_layer)).unwrap();
        }
        (Some(otel_layer), Some(loki_layer)) => {
            tracing::subscriber::set_global_default(collector.with(otel_layer).with(loki_layer))
                .unwrap();
        }
    }
}
