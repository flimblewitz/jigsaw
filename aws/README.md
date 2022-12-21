# Running Thespian in AWS
## CloudFormation
These AWS CloudFormation templates are used to set up the AWS resources required to run Thespian in ECS with ECS Service Connect enabled.

A CloudFormation stack using the `environment.yaml` template must be created once.

A CloudFormation stack using the `ecs_service.yaml` template must be created once for each Thespian that you want to run.

## Using an ALB
### Opting in
Optionally, you can choose to use an ALB to make it accessible to the outside world (although these templates only grant your own IP address access).

If you want to use an ALB, you must also go through some steps in AWS to allow the ALB to use HTTPS (see the `environment.yaml` description for details).

The benefit to using an ALB is that it gives you an easier way to test your services: you can just issue requests to `whatever-thespian-service-public-address-you-picked.whatever-domain-you-own.com` for a more realistic simulation of what a production experience would be like that.

If you're using `grpcurl` to test your services behind such an ALB using HTTPS, note that you have to ditch the `-plaintext` option and use `443` as the port, e.g.
```
grpcurl -proto ./proto/thespian.proto -import-path ./proto 'whatever-thespian-service-public-address-you-picked.whatever-domain-you-own.com:443' thespian.Thespian/A
```
### Opting out
You'll have to tunnel (via AWS Session Manager) into one of the EC2 instances serving as an ECS container instance, install `grpcurl` on it, and issue requests to `localhost` on that machine.

## ECS Service Connect
ECS Service Connect is like a rudimentary service mesh for ECS: it provides service discovery so that you can have inter-service traffic with dynamic addresses.

Imagine that you define a configuration for Thespian that's meant to play the role of a server. Let's name it `gilbert`.

Imagine that you define another configuration for Thespian that's meant to play the role of a client that calls `gilbert`. Let's name it `yorick`.

For local testing Thespian, you can just run one instance of `gilbert` on `http://localhost:6379` and configure `yorick` to make requests to `gilbert` with that exact address, and that's fine. The next step toward simulating real microservices at scale, though, is to find a "service discovery" solution that obviates the need to know in advance the exact IP addresses and ports that your microservices will be using.

The magic of service discovery solutions like ECS Service Connect is that you can abstract away the exact addresses and simply configure your microservice to hit a generic address that ends up resolving to any instance of the target microservice. So by creating an AWS CloudMap namespace and configuring our ECS Task Definition to use that namespace via ECS Service Connect, we can magically configure `yorick` to simply make requests to `http://gilbert:80`, and AWS will figure out how to route the inter-container traffic to some instance of `gilbert` that could be running on any IP or port.