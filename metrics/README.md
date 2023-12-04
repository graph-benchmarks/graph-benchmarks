# Metrics Server

This is the component which collects all performance metrics from k8s clusters.

## Building & Running

### Building

1. Building with Docker
   ```shell
   docker build -t <your_tag> .
   ```

2. Building standalone binary
   ```shell
   CGO_ENABLED=0 GOOS=linux go build -o metrics-server
   ```

### Configuring

You can configure the metrics server by editing `config.yaml` and populate the required fields. 

### Running

1. If you have previously configured with `config.yaml`, you can simply run with:
   ```shell
   ./metrics-server -config path/to/config.yaml
   ```

2. You can also pass commandline arguments by:
   ```shell
   ./metrics-server \
       -psql-host PSQL_HOST \
       -psql-port PSQL_PORT \
       -psql-username PSQL_USER \
       -psql-password PSQL_PASS \
       -psql-db PSQL_DB_NAME \
       -grpc-host GRPC_HOST \
       -grpc-port GRPC_PORT \
       -k8s-auth true
   ```

## Development

### SQL Table Schema

```
id (INT) | run_id (INT) | start_time (INT) | time_delta (INT) | pod_name char(256) | cpu_usage (FLOAT) | ram_usage (FLOAT) | power_usage (FLOAT) | interval (INT) 
```