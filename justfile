# these paths are relative to the justfile 
PATH_TO_PROTOFILE := "thespian_tonic_build/proto/thespian.proto"
PATH_TO_PROTOFILE_DIRECTORY := "thespian_tonic_build/proto"

PATH_TO_THESPIAN_LOCAL_LOGS_DIRECTORY := "thespian_local_logs"

DEFAULT_GRPC_METHOD := "thespian.Thespian/A"

IMAGE_NAME := "thespian:debian-buster-slim"
PATH_TO_DOCKERFILE := "dockerfiles/Dockerfile.debian-buster-slim"

# # this is for direct local tempo
# OTEL_BACKEND_PORT := "4317"
# this is for the grafana agent
OTEL_BACKEND_PORT := "3601"

# this is what `docker compose` ends up naming it
DOCKER_INSTRUMENTATION_NETWORK := "instrumentation_instrumentation"

SERVER_CONTAINER_NAME := "server_thespian"
SERVER_PORT := "6379"
SERVER_SERVICE_NAME := "starfox_simulator"
SERVER_CONFIG_JSON := '{\"grpc_method_a\":{\"tracing_name\":\"do_a_barrel_roll\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"listen_to_peppy\",\"duration_ms\":1000}}},{\"ConcurrentActions\":[{\"Sleep\":{\"tracing_name\":\"press L\",\"duration_ms\":2000}},{\"Sleep\":{\"tracing_name\":\"press R\",\"duration_ms\":1500,\"failure_chance\":0.1}},{\"Function\":{\"tracing_name\":\"accidentally unplug controller\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"instinctually yank the controller to physically pull the arwing into a barrel roll\",\"duration_ms\":1000,\"failure_chance\":0.1}}},{\"Action\":{\"Sleep\":{\"tracing_name\":\"unplug the cord, unintentionally harmonizing your scream with the sound of slippy perishing\",\"duration_ms\":1000,\"failure_chance\":0.1}}}]}}]},{\"Action\":{\"Sleep\":{\"tracing_name\":\"die horribly\",\"duration_ms\":1000,\"failure_chance\":0.1}}}]},\"grpc_method_b\":null,\"grpc_method_c\":null}'

CLIENT_CONTAINER_NAME := "client_thespian"
CLIENT_PORT := "6380"
CLIENT_SERVICE_NAME := "rob_playing_starfox"
# note that SERVER_HOST_NAME must be replaced before this can be used. It will vary between local and container setups
CLIENT_CONFIG_JSON := replace('{\"grpc_method_a\":{\"tracing_name\":\"engage fun subroutine\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"wiggle clamp hands in anticipation\",\"duration_ms\":1000}}},{\"ConcurrentActions\":[{\"Sleep\":{\"tracing_name\":\"excitedly crank more power into LED eyes\",\"duration_ms\":2000}},{\"CallService\":{\"tracing_name\":\"interface with starfox game\",\"service_address\":\"http://SERVER_HOST_NAME\",\"service_port\":\"SERVER_PORT\",\"grpc_method\":\"A\"}}]},{\"Action\":{\"Sleep\":{\"tracing_name\":\"spin torso 180 degrees and pat self on back\",\"duration_ms\":1000}}}]},\"grpc_method_b\":null,\"grpc_method_c\":null}',"SERVER_PORT",SERVER_PORT)

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

# starts a docker network with grafana, loki, tempo, and the grafana agent
instrumentation:
  cd instrumentation && \
  THESPIAN_LOCAL_LOGS_DIRECTORY={{justfile_directory()}}/{{PATH_TO_THESPIAN_LOCAL_LOGS_DIRECTORY}} \
  docker compose up -d

# starts a thespian on localhost. Assumes instrumentation is up
# writes logs into a single corresponding file in a folder inside of this repo
local port service_name config_json:
  mkdir -p {{justfile_directory()}}/{{PATH_TO_THESPIAN_LOCAL_LOGS_DIRECTORY}}
  OTEL_BACKEND_ADDRESS="http://localhost:{{OTEL_BACKEND_PORT}}" \
  PORT={{port}} \
  SERVICE_NAME={{service_name}} \
  CONFIG_JSON="{{config_json}}" \
  cargo run > {{justfile_directory()}}/{{PATH_TO_THESPIAN_LOCAL_LOGS_DIRECTORY}}/{{service_name}}.log

@server_local:
  just local \
    {{SERVER_PORT}} \
    {{SERVER_SERVICE_NAME}} \
    '{{SERVER_CONFIG_JSON}}'

@client_local:
  just local \
    {{CLIENT_PORT}} \
    {{CLIENT_SERVICE_NAME}} \
    '{{replace(CLIENT_CONFIG_JSON,"SERVER_HOST_NAME","localhost")}}'

# starts a thespian container bound to localhost. Assumes instrumentation is up
# note that the container is being added to the instrumentation network
# note that the container is being given a label of "thespian". This allows the grafana agent to ignore other containers' logs
container port service_name config_json container_name:
  docker run \
    -d \
    -p {{port}}:{{port}} \
    -e OTEL_BACKEND_ADDRESS="http://grafana_agent:{{OTEL_BACKEND_PORT}}" \
    -e PORT={{port}} \
    -e SERVICE_NAME={{service_name}} \
    -e CONFIG_JSON="{{config_json}}" \
    --network {{DOCKER_INSTRUMENTATION_NETWORK}} \
    --name {{container_name}} \
    --label thespian \
    {{IMAGE_NAME}}

@server_container:
  just container {{SERVER_PORT}} \
    {{SERVER_SERVICE_NAME}} \
    '{{SERVER_CONFIG_JSON}}' \
    {{SERVER_CONTAINER_NAME}}

@client_container:
  just container {{CLIENT_PORT}} \
    {{CLIENT_SERVICE_NAME}} \
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
  docker exec -it {{container_name}} /bin/bash