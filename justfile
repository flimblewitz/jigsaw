image := "thespian:debian-buster-slim"
otel_backend_port := "4317"
loki_port := "3100"
docker_instrumentation_network := "instrumentation_thespian_instrumentation" # this is just what `docker compose` ends up naming it
server_container_name := "server_thespian"
client_container_name := "client_thespian"
server_port := "6379"
client_port := "6380"
server_config_json := '{\"service_name\":\"starfox_simulator\",\"grpc_method_a\":{\"tracing_name\":\"do_a_barrel_roll\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"listen_to_peppy\",\"duration_ms\":1000}}},{\"ConcurrentActions\":[{\"Sleep\":{\"tracing_name\":\"press L\",\"duration_ms\":2000}},{\"Sleep\":{\"tracing_name\":\"press R\",\"duration_ms\":1500,\"failure_chance\":0.5}},{\"Function\":{\"tracing_name\":\"accidentally unplug controller\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"instinctually yank the controller to physically pull the arwing into a barrel roll\",\"duration_ms\":1000,\"failure_chance\":0.4}}},{\"Action\":{\"Sleep\":{\"tracing_name\":\"unplug the cord, unintentionally harmonizing your scream with the sound of slippy perishing\",\"duration_ms\":1000,\"failure_chance\":0.3}}}]}}]},{\"Action\":{\"Sleep\":{\"tracing_name\":\"die horribly\",\"duration_ms\":1000,\"failure_chance\":0.1}}}]},\"grpc_method_b\":null,\"grpc_method_c\":null}'

default:
  # the "server" thespian is a "starfox_simulator" that has no downstream services. You can call its A method and it'll just write observability information
  # the "client" thespian is a "rob_playing_starfox" that tries to call the "starfox_simulator" thespian as its singular downstream service
  just --list

image:
  # --progress plain writes out all output instead of collapsing it in a nice colored animation. It's nice for thorough debugging
  docker build -t {{image}} -f dockerfiles/Dockerfile.debian-buster-slim --progress plain .

# starts a docker network with grafana, loki, and tempo running in it
instrumentation:
  cd instrumentation && docker compose up -d

# starts a thespian on localhost. Assumes instrumentation is up
run PORT CONFIG_JSON:
  OTEL_BACKEND_ADDRESS="http://localhost:{{otel_backend_port}}" \
  LOKI_ADDRESS="http://localhost:{{loki_port}}" \
  PORT={{PORT}} \
  CONFIG_JSON="{{CONFIG_JSON}}" \
  cargo run

@run_server:
  just run {{server_port}} '{{server_config_json}}'

@run_client:
  # the CONFIG_JSON differs from the container_client recipe in that it has \"service_address\":\"http://localhost\"
  just run {{client_port}} '{\"service_name\":\"rob_playing_starfox\",\"grpc_method_a\":{\"tracing_name\":\"engage fun subroutine\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"wiggle clamp hands in anticipation\",\"duration_ms\":1000}}},{\"ConcurrentActions\":[{\"Sleep\":{\"tracing_name\":\"excitedly crank more power into LED eyes\",\"duration_ms\":2000}},{\"CallService\":{\"tracing_name\":\"interface with starfox game\",\"service_address\":\"http://localhost\",\"service_port\":\"{{server_port}}\",\"grpc_method\":\"A\"}}]},{\"Action\":{\"Sleep\":{\"tracing_name\":\"spin torso 180 degrees and pat self on back\",\"duration_ms\":1000}}}]},\"grpc_method_b\":null,\"grpc_method_c\":null}'


# starts a containerized thespian bound to localhost. Assumes instrumentation is up
container PORT CONFIG_JSON container_name:
  # note that containers in a docker network must make requests to other containers in the network using their container names as addresses. That's why we don't use localhost in these environment variables
  OTEL_BACKEND_ADDRESS="http://tempo:{{otel_backend_port}}" \
  LOKI_ADDRESS="http://loki:{{loki_port}}" \
  PORT={{PORT}} \
  CONFIG_JSON="{{CONFIG_JSON}}" \
  docker run -d -p {{PORT}}:{{PORT}} -e OTEL_BACKEND_ADDRESS -e LOKI_ADDRESS -e PORT -e CONFIG_JSON --network {{docker_instrumentation_network}} --name {{container_name}} {{image}}

@container_server:
  just container 6379 '{{server_config_json}}' {{server_container_name}}

@container_client:
  # the CONFIG_JSON differs from the run_client recipe in that it has \"service_address\":\"http://{{server_container_name}}\"
  just container 6380 '{\"service_name\":\"rob_playing_starfox\",\"grpc_method_a\":{\"tracing_name\":\"engage fun subroutine\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"wiggle clamp hands in anticipation\",\"duration_ms\":1000}}},{\"ConcurrentActions\":[{\"Sleep\":{\"tracing_name\":\"excitedly crank more power into LED eyes\",\"duration_ms\":2000}},{\"CallService\":{\"tracing_name\":\"interface with starfox game\",\"service_address\":\"http://{{server_container_name}}\",\"service_port\":\"{{server_port}}\",\"grpc_method\":\"A\"}}]},{\"Action\":{\"Sleep\":{\"tracing_name\":\"spin torso 180 degrees and pat self on back\",\"duration_ms\":1000}}}]},\"grpc_method_b\":null,\"grpc_method_c\":null}' {{client_container_name}}


ping_https address:
  grpcurl -proto ./proto/thespian.proto -import-path ./proto {{address}}:443 thespian.Thespian/A

ping_local port:
  grpcurl -plaintext -proto ./proto/thespian.proto -import-path ./proto localhost:{{port}} thespian.Thespian/A

ping_server:
  just ping_local {{server_port}}

ping_client:
  just ping_local {{client_port}}


enter_container container_name:
  docker exec -it {{container_name}} /bin/sh