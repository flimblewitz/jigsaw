# What is this?
This folder contains everything needed to start simple local instances of Grafana, Loki, Tempo, and the Grafana Agent that can be used in concert with local instances of Thespian.

# How to run
```
docker compose up
```
The `docker-compose.yaml` was heavily based on [an official example for running Tempo via Docker Compose](https://github.com/grafana/tempo/blob/main/example/docker-compose/local/docker-compose.yaml).

This gets you
- Grafana at http://localhost:3000
- Loki at http://localhost:3100
- Tempo at
  - http://localhost:3200 for queries
  - http://localhost:4317 for otlp grpc ingestion
- Grafana Agent at
  - http://localhost:12345 for interactions like /-/ready and /-/config
  - http://localhost:3601 for otlp grpc ingestion

# Grafana
The default grafana login is `admin`/`admin`.

The Grafana datasources file is based on [an official example for running Grafana with Loki, Tempo, and Prometheus via Docker Compose](https://github.com/grafana/tns/blob/main/production/docker-compose/datasources.yaml) named "TNS" (The New Stack).

The highlights are
- the baked-in registrations of Loki and Tempo as datasources
- the `derivedFields` setting that links logs in Loki to traces in Tempo
- the mapping of the `service_name` span attribute to `service.name`. In Tempo, `service.name` is the magic keyword for service names (spans won't be labeled nicely without it). However, Loki doesn't allow periods in labels, so that's why `service_name` is the actual attribute name being used <!-- TODO confirm that this is actually all true, especially after trying out the opentelemetry collector -->

# Loki
Wait until Loki is ready according to http://localhost:3100/ready.

I'm using Loki's super convenient [baked-in local configuration file](https://github.com/grafana/loki/blob/main/cmd/loki/loki-local-config.yaml) as recommended by [the official example for running the Grafana Agent locally via Docker Compose](https://github.com/grafana/agent/blob/main/example/docker-compose/docker-compose.yaml) and [the official "TNS" example](https://github.com/grafana/tns/blob/main/production/docker-compose/docker-compose.yml).
## Push a log manually
https://grafana.com/docs/loki/latest/api/#push-log-entries-to-loki
```
curl -XPOST \
  -H "Content-Type: application/json" \
   "localhost:3100/loki/api/v1/push" \
  -d '{
  "streams": [
    {
      "stream": {
        "example_label": "hoi"
      },
      "values": [
          [ "1662082288000000000", "tem" ]
      ]
    }
  ]
}' -i
```

# Tempo
The Tempo config file is based on an [official example](https://github.com/grafana/tempo/blob/main/example/docker-compose/shared/tempo.yaml) for [running locally via Docker Compose](https://github.com/grafana/tempo/blob/main/example/docker-compose/local/docker-compose.yaml).
## Push a trace span manually
https://grafana.com/docs/tempo/latest/api_docs/pushing-spans-with-http/
```
curl -X POST http://localhost:9411 -H 'Content-Type: application/json' -d '[{
 "id": "1234",
 "traceId": "0123456789abcdef",
 "timestamp": 1608239395286533,
 "duration": 100000,
 "name": "span from bash!",
 "tags": {
    "http.method": "GET",
    "http.path": "/api"
  },
  "localEndpoint": {
    "serviceName": "shell script"
  }
}]' -i
```
Check for its existence:
```
curl http://localhost:3200/api/traces/0123456789abcdef -i
```

# Grafana Agent
The docker compose config and the Grafana Agent config file were both based on an [official example](https://github.com/grafana/agent/tree/main/example/docker-compose).

Note that the main benefits to using a "collector" like the Grafana Agent as an intermediary between your microservices and observability backends like Loki and Tempo are that
- it allows you to factor out at least some of the work needed to record observability information from your microservices
- it automatically batches outgoing requests to reduce the burden on your observability backends

## Ingesting Logs
Grafana Agent supports both "pull" and "push" strategies for log ingestion.

Grafana Agent was at least implicitly designed to run in concert with containerized microservices as opposed to processes running directly on your host machine.

This makes things a little painful in the latter case, at least for the "pull" strategy.

It's worth noting that since Grafana Agent is at least partly an extension of another Grafana-managed app named Promtail, [its log-related configuration is directly based on Promtail](https://grafana.com/docs/agent/latest/static/configuration/logs-config/#logs_instance_config).

### Pulling Logs
Most of Grafana Agent's log ingestion integrations use the "pull" strategy.

For containerized microservices, log labels can be dynamically inferred from container-related metadata via integrations like [the one for docker](docker_sd_config).

For local services, there are no such dynamic inferences available. You have to explicitly name log file path patterns and any exact labels that you want to be attached to each log therein. Because of this, the Grafana Agent in this repo is configured to
- have a read-only bind mount (via docker-compose) to a log folder in this repo with the expectation that any local instances of Thespian will write their logs to files in that folder
- scrape two specific log files named for the two example Thespian instances defined in this repo

### Pushing Logs
Note that this isn't recommended because it foists an unnecessary infrastructure-level responsibility onto your app code, but Thespian used the "push" strategy initially, and the perspective may prove valuable.

[Grafana Agent can be configured to receive logs via the same "Push API" that Loki uses, just as Promtail can](https://grafana.com/docs/loki/latest/clients/promtail/configuration/#loki_push_api). If you use this log ingestion integration, your app has to personally push the logs to this API (such as by using [the `tracing-loki` crate](https://docs.rs/tracing-loki/latest/tracing_loki/index.html)).

Example:
```yaml
logs:
  configs:
  - name: default
    # ...
    scrape_configs:
    - job_name: grafana_agent_intermediary_loki_push_api
      loki_push_api:
        server:
          http_listen_port: 3500
        labels:
          push_server: grafana_agent
        use_incoming_timestamp: true
```