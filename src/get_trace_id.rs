use opentelemetry::trace::TraceContextExt; // opentelemetry::Context -> opentelemetry::trace::Span
use tracing_opentelemetry::OpenTelemetrySpanExt; // tracing::Span to opentelemetry::Context

// todo: consider including span id too in a tuple response so that you can go from tempo to specific logs within a trace (if that's even possible)
// todo: consider delegating this to a layer similar to what this issue is requesting https://github.com/tokio-rs/tracing/issues/1481 and this implementation of a layer that only uses println!() instead of actually adding fields/tags to the span: https://gist.github.com/loriopatrick/bd00116562084b1d61f105f31d31afe0
pub fn get_trace_id() -> String {
    tracing::Span::current()
        .context()
        .span()
        .span_context()
        .trace_id()
        .to_string()
}
