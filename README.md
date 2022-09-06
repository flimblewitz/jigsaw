# What is this?
Jigsaw is a `configurable` `mock` `grpc` api that emits tracing information.

# What is it for?
The idea is to use it as a building block when testing microservice and instrumentation architectures.

For instance, I want to test out self-made CI/CD pipelines for AWS ECS and Kubernetes, various service mesh approaches, and Jaeger and Grafana Tempo.

# But aren't there already sample microservices in the tutorials for all of those things?
Yeah, usually. But the microservices involved are always justifiably contrived, and I want to
- make my own microservice to prove to myself that I actually understand what's going on
- be able to create scenarios of arbitrary complexity to simulate
- not create a new codebase for each contrived microservice of my own design

# What do you mean by `configurable` and `mock`?
Jigsaw has 3 (number subject to change) unary grpc methods literally named `A`, `B`, and `C` that do nothing unless configured and ultimately respond with an empty grpc message.

You can create a configuration that defines the logic to perform for each method, and jigsaw will enact that logic when the method is invoked as well as emit corresponding tracing information.

I say that the logic represents `mock` behavior because it doesn't actually do anything other than either sleep or invoke a grpc method in another jigsaw instance. The sleep operations are loosely intended to simulate I/O but could also represent something less atomic.

For instance, you could have two jigsaw instances where one plays the role of a consumer-facing api and another plays the role of a database, and you could configure one of the grpc methods of the latter to sleep for a long time to simulate an expensive read/write query. Alternatively, you could have just the one jigsaw instance playing the role of the consumer-facing api and configure one of its grpc methods to have a sleep that simulates both the act of sending a request to a database _and_ the database performing the query itself. Whatever floats your boat.

For the sake of flexing your instrumentation's muscles by making jigsaw scenarios look more realistic, you can also configure actions to happen concurrently and with arbitrary levels of nested functions.

The configuration spec is defined by the types in [the jigsaw_instance.rs file](/src/jigsaw_instance.rs). You must supply your configuration as json via the `CONFIG_JSON` environment variable, and it will be deserialized by `serde` into those types.

# Why is every span named something generic like "Function.enact" or "Action.enact" instead of using the names for functions and actions I entered in my config?
It's a limitation of the `tracing` crate: it needs to use a hardcoded string (`&'static str`) for span names. There's no getting around it; span names just can't be dynamic.

But your custom names are still on the spans, they're just attributes/tags instead, so you'll have to click a bit more to see them (sorry).

# Is this done?
Nope.

Todo:
- preserve/propagate trace id across services
- timeouts for the service call action
- retries for actions
- chaos (chance of failure) for actions

---
# Development
The `tonic-build` build dependency uses `prost`, which requires `protoc`.

To get `protoc` on ubuntu, do this:
```apt install -y protobuf-compiler libprotobuf-dev```

## How to test locally
Start one or more instances of jigsaw with the `.vscode/launch.json`, and then do the following:
```./grpcurl -plaintext -proto ./proto/jigsaw.proto -import-path ./proto localhost:6379 jigsaw.Jigsaw/A```