# What is this?
Jigsaw is a `declaratively configurable` `mock` `grpc` api that emits tracing information.

# What is it for?
The idea is to use it as a building block when testing microservice and instrumentation architectures.

For instance, I want to test out self-made CI/CD pipelines for AWS ECS and Kubernetes, and I also want to try out Jaeger.

# But aren't there already sample microservices in the tutorials for all of those things?
Yeah, usually. But the microservices involved are always justifiably contrived, and I want to
- make my own microservice to prove to myself that I actually understand what's going on
- be able to create scenarios of arbitrary complexity to simulate
- not create a new codebase for each contrived microservice of my own design

# What do you mean by `declaratively configurable`?
A vanilla jigsaw instance has 3 (number subject to change) unary grpc methods literally named `A`, `B`, and `C` that respond with nothing and do nothing.

You can create a configuration that opaquely defines the internal logic to perform for each method - including invocations of grpc methods in *other jigsaw instances* - and jigsaw will enact that logic when the method is invoked as well as emit corresponding tracing information.

The grammar for configuration is forthcoming.

I plan to eventually include chaos (a chance of failure) in the configuration as well.

# What do you mean by `mock`?
Ultimately, jigsaw doesn't really _do_ anything when reacting to a stimulus other than sleep or invoke a grpc method in another jigsaw instance. The sleep operations are basically meant to simulate I/O.

For instance, one jigsaw instance might simulate the role of a database, and you could configure one of its grpc methods to sleep for a long time to simulate an expensive read/write.

---
# Development
The `tonic-build` build dependency uses `prost`, which requires `protoc`.

To get `protoc` on ubuntu, do this:
```apt install -y protobuf-compiler libprotobuf-dev```

## How to test locally
```./grpcurl -plaintext -proto ./proto/jigsaw.proto -import-path ./proto localhost:6379 jigsaw.Jigsaw/A```