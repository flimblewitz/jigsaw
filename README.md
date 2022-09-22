# What is this?
Jigsaw is an `instrumented` `configurable` `mock` gRPC API that can issue requests to other instances of itself.

# What is it for?
The idea is to use it as a building block when testing observability backends and microservice infrastructures.

For instance, I want to test out Grafana Loki and Grafana Tempo as observability backends, and I also want to test out AWS ECS and Kubernetes with various service mesh approaches. I also want to test Github Actions-based CI/CD pipelines in concert with these things.

# But aren't there already sample microservices in the tutorials for all of those things?
Yeah, usually. But the microservices involved are always contrived, and I find that distracting.

Instead, I want to
- make my own microservice to prove to myself that I actually understand rust, gRPC, distributed tracing, etc.
- be able to create scenarios of arbitrary shape to simulate when testing
- not create a new codebase for each simulated microservice

# What do you mean by `instrumented`?
[OpenTelemetry distributed tracing](https://opentelemetry.io/docs/concepts/observability-primer/#understanding-distributed-tracing) information is emitted to observability backends. The official site explains it best, but in short, the concepts are
- `span event`: a log
- `span`: a "span" of time for a specific code execution of interest, including code context and any associated span events. Spans can be nested in other spans
- `trace`: a tree of spans starting from a "root" span

In this repo's [instrumentation](instrumentation) folder, there are instructions to locally run two observability backends as well as a nice UI for them:
- [Grafana](https://grafana.com/grafana/) is the UI
- [Grafana Loki](https://grafana.com/oss/loki/) lets you search your span events
- [Grafana Tempo](https://grafana.com/oss/tempo/) assembles traces

## What rust crates facilitate that?
- [`tracing`](https://docs.rs/tracing/latest/tracing/) lets us create spans and span events in and around our code
- [`opentelemetry`](https://docs.rs/opentelemetry/latest/opentelemetry/) lets us propagate distributed tracing context along inbound and outbound requests so that spans can encapsulate microservice interactions
- [`tracing_opentelemetry`](https://docs.rs/tracing-opentelemetry/latest/tracing_opentelemetry/) lets us exchange distributed tracing context between `tracing` and `opentelemetry`
- [`tracing_subscriber`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/index.html) lets us actually collect spans and span events created by `tracing`. It also lets us write span events to stdout if desired
- [`opentelemetry_otlp`](https://docs.rs/opentelemetry-otlp/latest/opentelemetry_otlp/) lets us emit collected spans and span events to an OpenTelemetry-compliant observability backend (e.g. Grafana Tempo)
- [`tracing_loki`](https://docs.rs/tracing-loki/latest/tracing_loki/) lets us emit collected span events to Grafana Loki specifically

### Among those rust crates and their interactions, are there any quirks worth noting?
Yeah.
- in order to associate each individual span with its trace, I had to make a [`get_trace_id`](src/get_trace_id.rs) helper function and explicitly instrument every function with its output
- `tracing_loki` automatically adds a label for the log `level`, which is against the official best practices for Grafana Loki

# What do you mean by `configurable` and `mock`?
Jigsaw has 3 (number subject to change) unary gRPC methods literally named `A`, `B`, and `C` that do nothing by default and [use an empty gRPC message as input and output](proto/jigsaw.proto).

I say that it's `configurable` because you can define logic to simulate inside each of those methods, and Jigsaw will enact that logic when the method is invoked as well as emit corresponding spans and span events.

I say that it's a `mock` API because it doesn't ultimately do anything other than either sleep or invoke a gRPC method in another Jigsaw instance. The sleep operations are loosely intended to simulate I/O but could also represent something less atomic: it's all up to your imagination.

For instance, you could have two Jigsaw instances where one plays the role of a consumer-facing api and another plays the role of a database, and you could configure one of the gRPC methods of the latter to sleep for a long time to simulate an expensive read/write query. Alternatively, you could have just the one Jigsaw instance playing the role of the consumer-facing api and configure one of its gRPC methods to have a sleep that simulates both the act of sending a request to a database _and_ the database performing the query itself. Whatever floats your boat.

For the sake of flexing your instrumentation's muscles by making Jigsaw scenarios look more realistic, you can also configure actions to happen concurrently and with arbitrary levels of nested functions.

## How do I configure it?
Use the following environment variables:
- `PORT`: the port Jigsaw runs on
- `CONFIG_JSON`: this defines the behavior for Jigsaw's three gRPC methods. The specification is defined by the types in [the jigsaw_instance.rs file](/src/jigsaw_instance.rs). There are examples in [the example_configs folder](example_configs)
- `OTEL_BACKEND_ADDRESS`: this defines the gRPC address of the OpenTelemetry backend (e.g. Tempo) to which your Jigsaw instance should send its spans and span events. The default value is `http://localhost:4317`
- `LOKI_ADDRESS`: if you want Jigsaw to send span events directly to a Loki instance, fill this in. Example: `http://localhost:3200`

## Can it be configured to fail sometimes?
Yes! While testing out happy paths with distributed tracing is well and good, for a more complete experience, you probably ought to see what happens if errors occur. Jigsaw's configuration has a `failure_chance` property that you can optionally include for those sleep actions mentioned previously.

If you get a "low roll" for any sleep action in a gRPC method, the gRPC request will result in a response with the `INTERNAL` code (representing an internal error) instead of `OK`.

### How are error logs (span events) handled?
_Without_ tracing, I would personally generally deal with an unexpected error by logging it at the **highest possible level** - _the place where you ultimately map the error to some sort of response/return value_ - along with
- a stack trace
- relevant high-level context

_With_ tracing, assuming you're instrumenting most/all of your functions, it actually makes more sense to emit an error span event at the **lowest possible level** - _the place where the error actually originates_ - because the corresponding trace will automatically include
- a "span trace" (analogous to a stack trace)
- all the context you allowed to be included in each span

In accordance with this thinking, Jigsaw emits an error span event _immediately_ after any failure occurs and then lets the error bubble up for response handling but no further logging.

## Why are spans named something generic like "Function.enact" or "sleep" instead of the `tracing_name`s I entered in my config?
It's a limitation of the `tracing` crate: it needs to use hardcoded strings (`&'static str`) for span names. There's no getting around it; Jigsaw can't dynamically give spans names based on your configuration, so it gives them generic names instead.

However, **your custom names are still on the spans**! They're just attributes with a key of `_tracing_name` instead (the leading underscore makes them come first alphabetically), so when you're looking at the UI of your distributed tracing backend (e.g. Grafana Tempo), you'll have to do some extra clicking in order to see them.

# Is this done?
Nope.

Todo:
- timeouts for the service call action
- retries for actions
- [Tempo metrics](https://grafana.com/docs/tempo/latest/metrics-generator/) (APM dashboard, metrics from spans, and a service graph). The [example setups](https://grafana.com/docs/tempo/latest/getting-started/example-demo-app/) probably contain example configurations that enable them
- AWS ECS and ALB
- github actions CI/CD. Maybe Dagger too?
- kubernetes and linkerd/traefik mesh, then again with AWS EKS
- try `jigsaw -> otel collector -> loki/tempo` instead of `jigsaw -> loki/tempo` because the former seems to be more realistic and the latter relies on the quirky `tracing-loki` crate
- take a good, hard look at all those `todo` comments and auxiliary READMEs
- probably rename this to `thespian` because it frankly seems like a better name in every regard

---
# Development
## Requirements
The `tonic-build` build dependency uses the `prost` crate, which requires [`protoc`](https://grpc.io/docs/protoc-installation/).

## How to test locally
If you're using vscode, there are [launch configs](.vscode/launch.json) that you can use to kick off debugging sessions.

To issue gRPC requests, [`grpcurl`](https://github.com/fullstorydev/grpcurl) is pretty simple. For example:
```
./grpcurl -plaintext -proto ./proto/jigsaw.proto -import-path ./proto localhost:6379 jigsaw.Jigsaw/A
```