default:
  just --list

instrumentation:
  echo "starting grafana, loki, and tempo in the background"
  cd instrumentation && docker compose up -d

### todo: determine a nice way to get every recipe, including the following commented-out ones, running in the background but with a way to clean them up (maybe with another recipe). These doesn't work as is because they try to synchronously run commands, but `cargo run` doesn't terminate when it starts. I'm not going to worry about that for now
# all: instrumentation standalone call_service
# thespians: standalone call_service

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