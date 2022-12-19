default:
  just --list

image:
  docker build -t thespian:rust-slim -f Dockerfile.rust-slim .

instrumentation:
  echo "starting grafana, loki, and tempo in the background"
  cd instrumentation && docker compose up -d

standalone_container:
  # instrumentation containers must be running first
  OTEL_BACKEND_ADDRESS="http://tempo:4317" \
  LOKI_ADDRESS="http://loki:3100" \
  PORT="6379" \
  CONFIG_JSON="{\"service_name\":\"starfox_simulator\",\"grpc_method_a\":{\"tracing_name\":\"do_a_barrel_roll\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"listen_to_peppy\",\"duration_ms\":1000}}},{\"ConcurrentActions\":[{\"Sleep\":{\"tracing_name\":\"press L\",\"duration_ms\":2000}},{\"Sleep\":{\"tracing_name\":\"press R\",\"duration_ms\":1500,\"failure_chance\":0.5}},{\"Function\":{\"tracing_name\":\"accidentally unplug controller\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"instinctually yank the controller to physically pull the arwing into a barrel roll\",\"duration_ms\":1000,\"failure_chance\":0.4}}},{\"Action\":{\"Sleep\":{\"tracing_name\":\"unplug the cord, unintentionally harmonizing your scream with the sound of slippy perishing\",\"duration_ms\":1000,\"failure_chance\":0.3}}}]}}]},{\"Action\":{\"Sleep\":{\"tracing_name\":\"die horribly\",\"duration_ms\":1000,\"failure_chance\":0.1}}}]},\"grpc_method_b\":null,\"grpc_method_c\":null}" \
  docker run -d -p 6379:6379 -e OTEL_BACKEND_ADDRESS -e LOKI_ADDRESS -e PORT -e CONFIG_JSON --network instrumentation_thespian_instrumentation --name standalone_thespian thespian:rust-slim

call_service_container:
  # instrumentation containers must be running first
  OTEL_BACKEND_ADDRESS="http://tempo:4317" \
  LOKI_ADDRESS="http://loki:3100" \
  PORT="6380" \
  CONFIG_JSON="{\"service_name\":\"rob_playing_starfox\",\"grpc_method_a\":{\"tracing_name\":\"engage 'fun' subroutine\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"wiggle clamp hands in anticipation\",\"duration_ms\":1000}}},{\"ConcurrentActions\":[{\"Sleep\":{\"tracing_name\":\"excitedly crank more power into LED eyes\",\"duration_ms\":2000}},{\"CallService\":{\"tracing_name\":\"interface with starfox game\",\"service_address\":\"http://127.0.0.1\",\"service_port\":\"6379\",\"grpc_method\":\"A\"}}]},{\"Action\":{\"Sleep\":{\"tracing_name\":\"spin torso 180 degrees and pat self on back\",\"duration_ms\":1000}}}]},\"grpc_method_b\":null,\"grpc_method_c\":null}" \
  docker run -d -p 6380:6380 -e OTEL_BACKEND_ADDRESS -e LOKI_ADDRESS -e PORT -e CONFIG_JSON --network instrumentation_thespian_instrumentation --name call_service_thespian thespian:rust-slim

standalone:
  echo "starting the starfox_simulator thespian (has no downstream services)"
  LOKI_ADDRESS="http://localhost:3100" \
  PORT="6379" \
  CONFIG_JSON="{\"service_name\":\"starfox_simulator\",\"grpc_method_a\":{\"tracing_name\":\"do_a_barrel_roll\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"listen_to_peppy\",\"duration_ms\":1000}}},{\"ConcurrentActions\":[{\"Sleep\":{\"tracing_name\":\"press L\",\"duration_ms\":2000}},{\"Sleep\":{\"tracing_name\":\"press R\",\"duration_ms\":1500,\"failure_chance\":0.5}},{\"Function\":{\"tracing_name\":\"accidentally unplug controller\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"instinctually yank the controller to physically pull the arwing into a barrel roll\",\"duration_ms\":1000,\"failure_chance\":0.4}}},{\"Action\":{\"Sleep\":{\"tracing_name\":\"unplug the cord, unintentionally harmonizing your scream with the sound of slippy perishing\",\"duration_ms\":1000,\"failure_chance\":0.3}}}]}}]},{\"Action\":{\"Sleep\":{\"tracing_name\":\"die horribly\",\"duration_ms\":1000,\"failure_chance\":0.1}}}]},\"grpc_method_b\":null,\"grpc_method_c\":null}" \
  cargo run

call_service:
  echo "starting the rob_playing_starfox thespian (has one downstream service)"
  LOKI_ADDRESS="http://localhost:3100" \
  PORT="6380" \
  CONFIG_JSON="{\"service_name\":\"rob_playing_starfox\",\"grpc_method_a\":{\"tracing_name\":\"engage 'fun' subroutine\",\"operations\":[{\"Action\":{\"Sleep\":{\"tracing_name\":\"wiggle clamp hands in anticipation\",\"duration_ms\":1000}}},{\"ConcurrentActions\":[{\"Sleep\":{\"tracing_name\":\"excitedly crank more power into LED eyes\",\"duration_ms\":2000}},{\"CallService\":{\"tracing_name\":\"interface with starfox game\",\"service_address\":\"http://127.0.0.1\",\"service_port\":\"6379\",\"grpc_method\":\"A\"}}]},{\"Action\":{\"Sleep\":{\"tracing_name\":\"spin torso 180 degrees and pat self on back\",\"duration_ms\":1000}}}]},\"grpc_method_b\":null,\"grpc_method_c\":null}" \
  cargo run

ping_standalone:
  ./grpcurl -plaintext -proto ./proto/thespian.proto -import-path ./proto localhost:6379 thespian.Thespian/A

ping_call_service:
  ./grpcurl -plaintext -proto ./proto/thespian.proto -import-path ./proto localhost:6380 thespian.Thespian/A

enter_standalone_container:
  docker exec -it standalone_thespian /bin/sh

enter_call_service_container:
  docker exec -it call_service_thespian /bin/sh