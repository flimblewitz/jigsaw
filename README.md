# What is this?
Thespian is an `instrumented` `gRPC` API that can act out arbitrary configured behavior and issue empty requests to other instances of itself.

# What is it for?
You can use it as a building block when testing observability backends for instrumentation and microservice infrastructures.

For instance, I want to test out Grafana Loki and Grafana Tempo as observability backends, and I also want to test out AWS ECS and Kubernetes with various service mesh approaches. I also want to test Github Actions-based CI/CD pipelines in concert with these things.

You can also use it as a seed project (a sort of template or starting point from which to make other structurally similar projects).

# But aren't there already sample microservices in tutorials for those things?
Yeah, usually. But the microservices involved are always very contrived, and I find that distracting. Rather than having to scrutinize one or more very particular or whimsical sample microservice, I would prefer be able to play with something minimal that has _nothing tangible going on_ and the ability to be freely instantiated with configurable "pretend" behavior that can be whatever I want it to be.

Thespian is motivated by the desires to
- be able to simulate microservice ecosystems and interactions of arbitrary shape
- not have to create or understand a new codebase for each simulated microservice

# What do you mean by `instrumented`?
[OpenTelemetry distributed tracing](https://opentelemetry.io/docs/concepts/observability-primer/#understanding-distributed-tracing) information is emitted to observability backends. The official site explains it best, but in short, the concepts are
- `span event`: a log
- `span`: a window of time encapsulating a specific code execution of interest that includes code context and any associated span events. Spans can be nested in other spans
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

# What do you mean by "act out arbitrary configured behavior"?
Thespian has 3 unary gRPC methods literally named `A`, `B`, and `C` that do nothing by default and [use an empty gRPC message as input and output](proto/thespian.proto).

You can define "arbitrary configured behavior" to simulate inside each of those methods, and Thespian will act out that behavior when the method is invoked as well as emit corresponding spans and span events.

I say that it's "acting" because it ultimately doesn't do anything other than either sleep or invoke a gRPC method in another Thespian instance. The sleep operations are loosely intended to simulate I/O but could also represent something less atomic: it's all up to your imagination.

For instance, you could have two Thespian instances where one plays the role of a consumer-facing api and another plays the role of a database, and you could configure one of the gRPC methods of the latter to sleep for a long time to simulate an expensive read/write query. Alternatively, you could have just the one Thespian instance playing the role of the consumer-facing api and configure one of its gRPC methods to have a sleep that simulates both the act of sending a request to a database _and_ the database performing the query itself. Whatever floats your boat.

For the sake of flexing your instrumentation's muscles by making Thespian scenarios look more realistic, you can also configure actions to happen concurrently and with arbitrary levels of nested functions.

## How do I configure it?
Use the following environment variables:
- `PORT`: the port Thespian runs on
- `SERVICE_NAME`: the service name that Thespian includes in its spans and span events. This should be unique per Thespian instance
- `CONFIG_JSON`: this defines the behavior for Thespian's three gRPC methods. The specification is defined by the types in [the thespian_instance.rs file](/src/thespian_instance.rs). There are examples in [the example_configs folder](example_configs)
- `OTEL_BACKEND_ADDRESS`: this defines the gRPC address of the OpenTelemetry backend (e.g. Tempo) to which your Thespian instance should send its spans and span events. The default value is `http://localhost:4317`
- `LOKI_ADDRESS`: if you want Thespian to send span events directly to a Loki instance, fill this in. Example: `http://localhost:3200`

## Can it be configured to fail sometimes?
Yes! While testing out happy paths with distributed tracing and networking architectures (e.g. service meshes) is well and good, for a more complete experience, you probably ought to see what happens if errors occur. Thespian's configuration has a `failure_chance` property that you can optionally include for those sleep actions mentioned previously.

If you get a "low roll" for any sleep action in a gRPC method, the gRPC request will result in a response with the `INTERNAL` code (representing an internal error) instead of `OK`.

### How are error logs (span events) handled?
_Without_ tracing, I would personally generally deal with an unexpected error by logging it at the **highest possible level** - _the place where you ultimately map the error to some sort of response/return value_ - along with
- a stack trace
- relevant high-level context

_With_ tracing, assuming you're instrumenting most/all of your functions, it actually makes more sense to emit an error span event at the **lowest possible level** - _the place where the error actually originates_ - because the corresponding trace will automatically include
- a "span trace" (analogous to a stack trace)
- all the context you allowed to be included in each span

In accordance with this thinking, Thespian emits an error span event _immediately_ after any failure occurs and then lets the error bubble up for response handling but no further logging.

## Why are spans named something generic like "Function.enact" or "sleep" instead of the `tracing_name`s I entered in my config?
It's a limitation of the `tracing` crate: it needs to use hardcoded strings (`&'static str`) for span names. There's no getting around it; Thespian can't dynamically give spans names based on your configuration, so it gives them generic names instead.

However, **your custom names are still on the spans**! They're just attributes with a key of `_tracing_name` instead (the leading underscore makes them come first alphabetically), so when you're looking at the UI of your distributed tracing backend (e.g. Grafana Tempo), you'll have to do some extra clicking in order to see them.

# Is this done?
The most important parts for instrumentation are done, so it's sufficient for local use. service meshing with ECS Service Connect is functional, but I want to create a proof of concept for Kubernetes as well. CI/CD for microservice architectures are still in the pipeline.

## MVP
- [x] ingest the config from an environment variable
- [x] issue gRPC requests to other Thespian instances
- [x] create docker-compose.yaml for Grafana, Loki, and Tempo
- [x] emit tracing information
- [x] preserve/propagate trace id across services
- [x] configurable chaos (`failure_chance`)
## Instrumentation
- [ ] review the instrumentation `docker-compose.yaml` against [the Grafana "TNS" (The New Stack) example](https://github.com/grafana/tns/blob/main/production/docker-compose/docker-compose.yml)
- [ ] explore Tempo's [TraceQL](https://grafana.com/docs/tempo/latest/traceql/) feature for trace discovery, which [requires Parquet format to be enabled as is the default behavior of Tempo 2.0](https://grafana.com/docs/tempo/latest/configuration/parquet/)
- [ ] add Prometheus and configure Tempo to use it to enable [Tempo metrics](https://grafana.com/docs/tempo/latest/metrics-generator/) (APM dashboard, metrics from spans, and a service graph). The [example setups](https://grafana.com/docs/tempo/latest/getting-started/example-demo-app/) probably contain example configurations that enable them. It may just come down to the commented-out "metrics generator" lines in the existing Tempo config file
- [ ] stop sending spans and logs directly to Tempo and Loki and instead start using the Grafana Agent as an intermediary based on [the official Docker Compose example](https://github.com/grafana/agent/blob/main/example/docker-compose/README.md) and whatever other information is relevant in [the official setup documentation](https://grafana.com/docs/agent/latest/set-up/). Also note that this will mean retiring the `tracing-loki` crate
- [ ] enable [node_exporter](https://grafana.com/docs/agent/latest/configuration/integrations/node-exporter-config/) functionality to the Grafana Agent (it might be as easy as [a single line](https://github.com/grafana/agent/blob/main/example/docker-compose/agent/config/agent.yaml#LL86C4-L86C17), [two lines](https://grafana.com/grafana/dashboards/12558-node-exporter-from-agent-integration/), or [another container](https://github.com/grafana/tns/blob/main/production/docker-compose/docker-compose.yml#L121))
## Service Mesh
- [x] AWS ALB and ECS with ECS Service Connect
- [ ] local kubernetes
- [ ] linkerd
- [ ] AWS EKS
## Polish
- [x] add `just` integration to kick it all off faster
- [x] separate the `tonic-build` stuff into a [cargo workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) so that it doesn't slow down the build or perhaps separate into crates like [Rust on Nails](https://rust-on-nails.com/docs/api/grpc/)
- [ ] try to eliminate the need for the docker compose network because official examples don't use one
- [ ] find way to clean up background jobs initiated by `just`
- [ ] add container or script that just pokes Thespian over and over
- [ ] take a good, hard look at all those `todo/TODO` comments and auxiliary READMEs
- [ ] introduce the Opentelemetry Collector as an alternative to the Grafana Agent
## CI/CD
- [ ] github actions
## Tentative
- [ ] configurable timeouts for the service call action at the client level
- [ ] configurable retries for actions at the client level
- [ ] jaeger as an alternative to tempo (make sure to enable otlp, which uses 4317. See [the collector component's definition](https://www.jaegertracing.io/docs/1.38/deployment/#all-in-one))
- [ ] CI/CD using [Dagger](https://dagger.io)

# Development
## Requirements
The `tonic-build` build dependency uses the `prost` crate, which requires [`protoc`](https://grpc.io/docs/protoc-installation/).

See https://github.com/hyperium/tonic/tree/master/examples#examples for possible examples of what Ubuntu and Alpine need.

## How to test locally
If you're using vscode, there are [launch configs](.vscode/launch.json) that you can use to kick off debugging sessions.

To issue gRPC requests, [`grpcurl`](https://github.com/fullstorydev/grpcurl) is pretty simple. For example:
```
grpcurl -plaintext -proto ./proto/thespian.proto -import-path ./proto localhost:6379 thespian.Thespian/A
```