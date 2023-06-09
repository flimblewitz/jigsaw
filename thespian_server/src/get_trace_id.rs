use opentelemetry::trace::TraceContextExt; // opentelemetry::Context -> opentelemetry::trace::Span
use tracing_opentelemetry::OpenTelemetrySpanExt; // tracing::Span -> opentelemetry::Context

pub fn get_trace_id() -> String {
    tracing::Span::current()
        .context()
        .span()
        .span_context()
        .trace_id()
        .to_string()
}
