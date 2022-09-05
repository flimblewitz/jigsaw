# How to run
```
docker compose up
```

this gets you
- grafana at http://localhost:3000
- loki at http://localhost:3100
- jaeger at http://localhost:16686

wait until loki is ready according to http://localhost:3100/ready

steps to log into and deal with [grafana](http://localhost:3000):
- sign in as admin/admin
- add a data source in grafana with the url `http://loki:3100` (this resolves because of docker networking)

# Testing
## Loki
the loki setup comes from https://citizix.com/how-to-run-grafana-loki-with-docker-and-docker-compose/

the official getting started example has too much going on for my interests. I wish they had broken it up into steps starting from the bare minimum with just grafana and loki running purely locally
  https://grafana.com/docs/loki/latest/getting-started/

I'm not sure if this unexplained docker-compose.yaml runs defaults or not
  https://github.com/grafana/loki/blob/main/production/docker-compose.yaml
### Push a trace log manually 
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

### Deleting logs
This doesn't actually seem to be feasible, but it's probably worth delving deeper.

https://grafana.com/docs/loki/latest/api/#request-log-deletion

There's an automatic log entry deletion feature that's not documented very well.
- https://grafana.com/docs/loki/latest/operations/storage/logs-deletion/
- https://grafana.com/docs/loki/latest/operations/storage/retention/
- https://grafana.com/docs/loki/latest/operations/storage/filesystem/

## Tempo
I'm basing this off of two [official examples](https://grafana.com/docs/tempo/latest/getting-started/example-demo-app/)
- https://github.com/grafana/tempo/blob/main/example/docker-compose/loki
- https://github.com/grafana/tempo/blob/main/example/docker-compose/local
### Push a trace log manually
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

### Metrics generation
I disabled this, but the two example configs linked above have it enabled. It seems to require prometheus, which I didn't want to involve in this yet.