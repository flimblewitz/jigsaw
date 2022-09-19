# What is this?
Jigsaw is an `instrumented` `configurable` `mock` GRPC API that can issue requests to other instances of itself.

# What is it for?
The idea is to use it as a building block when testing observability backends and microservice infrastructure.

For instance, I want to test out Grafana Loki and Grafana Tempo as observability backends, and I also want to test out AWS ECS and Kubernetes with various service mesh approaches. I also want to test Github Actions-based CI/CD pipelines in concert with these things.

# But aren't there already sample microservices in the tutorials for all of those things?
Yeah, usually. But the microservices involved are always contrived (justifiably), and I find that distracting.

Instead, I want to
- make my own microservice to prove to myself that I actually understand what's going on with rust, GRPC, tracing, etc.
- be able to create scenarios of arbitrary shape to simulate when testing
- not create a new codebase for each contrived microservice of my own design

# What do you mean by `instrumented`?
OpenTelemetry [distributed tracing](https://opentelemetry.io/docs/concepts/observability-primer/#understanding-distributed-tracing) information is emitted to observability backends.

In other words,
- span events (**logs** that are tied to spans) are sent to [Grafana Loki](https://grafana.com/oss/loki/)
- spans are sent to [Grafana Tempo](https://grafana.com/oss/tempo/)

## What rust crates facilitate that?
- `tracing` lets us create spans and span events in and around our code
- `opentelemetry` lets us propagate distributed tracing context along inbound and outbound requests so that spans can encapsulate inter-service operations
- `tracing_opentelemetry` lets us exchange distributed tracing context between `tracing` and `opentelemetry`
- `tracing_subscriber` lets us actually collect spans and span events created by `tracing`. It also lets us log span events to stdout if we desire
- `opentelemetry_otlp` lets us emit collected spans and span events to an OpenTelemetry-compliant backend (e.g. Grafana Tempo)
- `tracing_loki` lets us emit collected span events to Grafana Loki specifically

# What do you mean by `configurable` and `mock`?
Jigsaw has 3 (number subject to change) unary GRPC methods literally named `A`, `B`, and `C` that do nothing by default and always ultimately respond with an empty GRPC message.

I say that it's `configurable` because you can define logic to simulate inside each of those methods, and Jigsaw will enact that logic when the method is invoked as well as emit corresponding spans and span events.

I say that it's a `mock` API because it doesn't ultimately do anything other than either sleep or invoke a GRPC method in another Jigsaw instance. The sleep operations are loosely intended to simulate I/O but could also represent something less atomic: it's all up to your imagination.

For instance, you could have two Jigsaw instances where one plays the role of a consumer-facing api and another plays the role of a database, and you could configure one of the GRPC methods of the latter to sleep for a long time to simulate an expensive read/write query. Alternatively, you could have just the one Jigsaw instance playing the role of the consumer-facing api and configure one of its GRPC methods to have a sleep that simulates both the act of sending a request to a database _and_ the database performing the query itself. Whatever floats your boat.

For the sake of flexing your instrumentation's muscles by making Jigsaw scenarios look more realistic, you can also configure actions to happen concurrently and with arbitrary levels of nested functions.

The configuration specification is defined by the types in [the jigsaw_instance.rs file](/src/jigsaw_instance.rs). You must supply your configuration as JSON via the `CONFIG_JSON` environment variable, and it will be deserialized by the `serde` crate into those types.

## Can it be configured to fail sometimes?
Yes! While testing out happy paths with distributed tracing is well and good, for a more complete experience, you probably ought to see what happens if errors occur. Jigsaw's configuration has a `failure_chance` property that you can optionally include for those sleep actions mentioned previously.

### But what does it actually do when it fails?
Without tracing, I would personally generally handle the logging of every kind of unexpected error by logging it exactly one time with a stack trace at the **highest possible level** - _the place where you truly handle the error in the code by mapping it to some sort of designated response/return value_ - and wrapping that error log in whatever context might be necessary to understand how it relates to that **highest possible level** for the sake of legibility in both the code and the app logs.

However, tracing turns this sort of flow upside down: if you emit an error event at the **lowest possible level** - _the place where it actually originates_ - then assuming you're instrumenting most/all of your functions, the log legibility will be perfectly sufficient because all the context you could ever need will simply exist in the corresponding trace, and the code legibility will actually be better because you're logging the error in the same place where it originates.

In accordance with this thinking, Jigsaw simply emits an error event _immediately_ after a failure occurs and then lets the error bubble up for response handling but no further logging.

## Why are spans named something generic like "Function.enact" or "sleep" instead of the `tracing_name`s I entered in my config?
It's a limitation of the `tracing` crate: it needs to use hardcoded strings (`&'static str`) for span names. There's no getting around it; Jigsaw can't dynamically give spans names based on your configuration, so it gives them generic names instead.

However, your custom names are still on the spans! They're just attributes with a key of `_tracing_name` instead (the leading underscore makes them come first alphabetically), so when you're looking at the UI of your distributed tracing backend (e.g. Grafana Tempo), you'll have to do some extra clicking in order to see them.

# Is this done?
Nope.

Todo:
- explain get_trace_id 
- timeouts for the service call action
- retries for actions
- AWS ECS and ALB
- github actions CI/CD
- kubernetes and linkerd/traefik mesh, then again with AWS EKS
- try `jigsaw -> otel collector -> loki/tempo` instead of `jigsaw -> loki/tempo` because the former seems to be more realistic and the latter relies on the quirky `tracing-loki` crate
- take a good, hard look at all those `todo` comments and auxiliary READMEs
- probably rename this to `thespian` because it frankly seems like a better name in every regard

---
# Development
The `tonic-build` build dependency uses the `prost` crate, which requires `protoc`.

To get `protoc` on ubuntu, do this:
```apt install -y protobuf-compiler libprotobuf-dev```

## How to test locally
Start one or more instances of Jigsaw with the `.vscode/launch.json`, and then do the following:
```./grpcurl -plaintext -proto ./proto/jigsaw.proto -import-path ./proto localhost:6379 jigsaw.Jigsaw/A```