# What is this?
This folder contains everything needed to start simple local instances of Grafana, Loki, and Tempo that can be used in concert with local instances of Jigsaw.

# How to run
```
docker compose up
```

This gets you
- Grafana at http://localhost:3000
- Loki at http://localhost:3100
- Tempo at http://localhost:3200

# Grafana
The default grafana login is `admin`/`admin`.

The configuration is based on two [official examples](https://grafana.com/docs/tempo/latest/getting-started/example-demo-app/)
- https://github.com/grafana/tempo/blob/main/example/docker-compose/loki
- https://github.com/grafana/tempo/blob/main/example/docker-compose/local
The highlights are
- the baked-in registrations of Loki and Tempo as datasources
- the `derivedFields` setting that links logs in the former to traces in the latter
- the mapping of the `service_name` span attribute to `service.name` in Tempo because that's the magic keyword for service names in Tempo (spans won't be labeled nicely without it). In contrast, Loki doesn't allow periods in labels, so that's why `service_name` is the actual attribute name being used

# Loki
Wait until Loki is ready according to http://localhost:3100/ready.

The Loki config file being used is based on https://citizix.com/how-to-run-grafana-loki-with-docker-and-docker-compose/

## Push a trace log manually 
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
## Push a trace log manually
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