PATH_TO_PROTOFILE := "proto/thespian.proto"
PATH_TO_PROTOFILE_DIRECTORY := "proto"
DEFAULT_GRPC_METHOD := "thespian.Thespian/A"

IMAGE_NAME := "thespian:debian-buster-slim"
PATH_TO_DOCKERFILE := "dockerfiles/Dockerfile.debian-buster-slim"

OTEL_BACKEND_PORT := "4317"
LOKI_PORT := "3100"

# this is what `docker compose` ends up naming it
DOCKER_INSTRUMENTATION_NETWORK := "instrumentation_thespian_instrumentation"

SERVER_CONTAINER_NAME := "server_thespian"
SERVER_PORT := "6379"
SERVER_CONFIG_JSON := '{\"service_name\":\"starfox_simulator\",\"grpc_method_a\":{\"tracing_name\":\"do_a_barrel_roll\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"listen_to_peppy\",\"duration_ms\":1000}}},{\"ConcurrentActions\":[{\"Sleep\":{\"tracing_name\":\"press L\",\"duration_ms\":2000}},{\"Sleep\":{\"tracing_name\":\"press R\",\"duration_ms\":1500,\"failure_chance\":0.5}},{\"Function\":{\"tracing_name\":\"accidentally unplug controller\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"instinctually yank the controller to physically pull the arwing into a barrel roll\",\"duration_ms\":1000,\"failure_chance\":0.4}}},{\"Action\":{\"Sleep\":{\"tracing_name\":\"unplug the cord, unintentionally harmonizing your scream with the sound of slippy perishing\",\"duration_ms\":1000,\"failure_chance\":0.3}}}]}}]},{\"Action\":{\"Sleep\":{\"tracing_name\":\"die horribly\",\"duration_ms\":1000,\"failure_chance\":0.1}}}]},\"grpc_method_b\":null,\"grpc_method_c\":null}'

CLIENT_CONTAINER_NAME := "client_thespian"
CLIENT_PORT := "6380"
# note that SERVER_HOST_NAME must be replaced before this can be used. It will vary between local and container setups
CLIENT_CONFIG_JSON := replace('{\"service_name\":\"rob_playing_starfox\",\"grpc_method_a\":{\"tracing_name\":\"engage fun subroutine\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"wiggle clamp hands in anticipation\",\"duration_ms\":1000}}},{\"ConcurrentActions\":[{\"Sleep\":{\"tracing_name\":\"excitedly crank more power into LED eyes\",\"duration_ms\":2000}},{\"CallService\":{\"tracing_name\":\"interface with starfox game\",\"service_address\":\"http://SERVER_HOST_NAME\",\"service_port\":\"SERVER_PORT\",\"grpc_method\":\"A\"}}]},{\"Action\":{\"Sleep\":{\"tracing_name\":\"spin torso 180 degrees and pat self on back\",\"duration_ms\":1000}}}]},\"grpc_method_b\":null,\"grpc_method_c\":null}',"SERVER_PORT",SERVER_PORT)

# this is what runs when you run `just` with no command specified
_default:
  @just --list 

build_image:
  # --progress plain writes out all output instead of collapsing it in a nice colored animation. It's nice for thorough debugging
  docker build \
    -t {{IMAGE_NAME}} \
    -f {{PATH_TO_DOCKERFILE}} \
    --progress plain \
    .

# starts a docker network with grafana, loki, and tempo
instrumentation:
  cd instrumentation && docker compose up -d

# starts a thespian on localhost. Assumes instrumentation is up
local port config_json:
  OTEL_BACKEND_ADDRESS="http://localhost:{{OTEL_BACKEND_PORT}}" \
  LOKI_ADDRESS="http://localhost:{{LOKI_PORT}}" \
  PORT={{port}} \
  CONFIG_JSON="{{config_json}}" \
  cargo run

@server_local:
  just local \
    {{SERVER_PORT}} \
    '{{SERVER_CONFIG_JSON}}'

@client_local:
  just local \
    {{CLIENT_PORT}} \
    '{{replace(CLIENT_CONFIG_JSON,"SERVER_HOST_NAME","localhost")}}'

# starts a thespian container bound to localhost. Assumes instrumentation is up
container port config_json container_name:
  # note that containers in a docker network must make requests to other containers in the network using their container names as addresses. That's why we don't use localhost in these environment variables
  OTEL_BACKEND_ADDRESS="http://tempo:{{OTEL_BACKEND_PORT}}" \
  LOKI_ADDRESS="http://loki:{{LOKI_PORT}}" \
  PORT={{port}} \
  CONFIG_JSON="{{config_json}}" \
  docker run \
    -d \
    -p {{port}}:{{port}} \
    -e OTEL_BACKEND_ADDRESS \
    -e LOKI_ADDRESS \
    -e PORT \
    -e CONFIG_JSON \
    --network {{DOCKER_INSTRUMENTATION_NETWORK}} \
    --name {{container_name}} \
    {{IMAGE_NAME}}

@server_container:
  just container {{SERVER_PORT}} \
    '{{SERVER_CONFIG_JSON}}' \
    {{SERVER_CONTAINER_NAME}}

@client_container:
  just container {{CLIENT_PORT}} \
    '{{replace(CLIENT_CONFIG_JSON,"SERVER_HOST_NAME",SERVER_CONTAINER_NAME)}}' \
    {{CLIENT_CONTAINER_NAME}}

# use this to ping thespian when it's remotely hosted with HTTPS
ping_https address:
  grpcurl \
    -proto {{justfile_directory()}}/{{PATH_TO_PROTOFILE}} \
    -import-path {{justfile_directory()}}/{{PATH_TO_PROTOFILE_DIRECTORY}} \
    {{address}}:443 \
    {{DEFAULT_GRPC_METHOD}}
 
ping_http address port:
  grpcurl \
    -proto {{justfile_directory()}}/{{PATH_TO_PROTOFILE}} \
    -import-path {{justfile_directory()}}/{{PATH_TO_PROTOFILE_DIRECTORY}} \
    -plaintext \
    {{address}}:{{port}} \
    {{DEFAULT_GRPC_METHOD}}

server_ping:
  just ping_http localhost {{SERVER_PORT}}

client_ping:
  just ping_http localhost {{CLIENT_PORT}}


enter_container container_name:
  docker exec -it {{container_name}} /bin/sh