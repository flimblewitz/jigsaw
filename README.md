# What is this?
Jigsaw is a `declaratively configurable` `mock` `grpc` api that emits tracing information.

# What is it for?
The idea is to use it to test instrumentation and container architectures. For instance, I want to test out self-made CI/CD pipelines for AWS ECS and Kubernetes, and I also want to try out Jaeger.

# But aren't there already tutorials for all of those things?
Yeah, absolutely. But the microservices involved are always rather contrived (which makes sense), and I want to prove to myself that I actually understand what's going on by making my own building block that enables simulations of arbitrary complexity. But I don't want to create a new repo/codebase for each contrived microservice I want to cook up for the testing exercises, hence the existence of one mock api with declaratively configurable behavior.

# What do you mean by `declaratively configurable`?
A vanilla jigsaw instance has 3 (number subject to change) unary grpc methods that respond with nothing and do nothing.

You can create a configuration that opaquely defines the internal logic to perform for each method - including invocations of grpc methods in *other microservices* - and jigsaw will perform that behavior when the method is invoked and emit corresponding tracing information.

The exact grammar for configuration is forthcoming.

I plan to eventually include chaos (fallibility) in the configuration as well.

# What do you mean by `mock`?
Ultimately, jigsaw doesn't really _do_ anything when reacting to a stimulus other than sleep or invoke a grpc method in another microservice. The sleep operations are basically meant to simulate I/O. For instance, one jigsaw instance might simulate the role of a database, and you could put a comparatively long sleep in one of its configured grpc methods to simulate a read/write.

---
# Development
The `tonic-build` build dependency uses `prost`, which requires `protoc`.

To get it on ubuntu, do this.
```apt install -y protobuf-compiler libprotobuf-dev```

## How to test locally
```./grpcurl -plaintext -proto ./proto/jigsaw.proto -import-path ./proto localhost:6379 jigsaw.Jigsaw/A```