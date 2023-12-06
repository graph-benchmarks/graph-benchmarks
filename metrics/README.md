# Metrics Server

This is the component which collects all performance metrics from k8s clusters.

Implemented metrics:

- [x] CPU usage (unit: K8s standard CPU Core)
- [x] Memory usage (unit: Ki)
- [ ] Power usage

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

You can configure the metrics server by copying the sample configuration file and populating the required fields.

```shell
cp example.config.yaml config.yaml
```

### Running

1. If you have previously configured with `config.yaml`, you can simply run with:
   ```shell
   ./metrics-server -config <path/to/config.yaml>
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

> [!IMPORTANT]
> The following SQL command is not tested and should be used for reference only.

```sql
CREATE TABLE `performance_metrics` (
    `id`         BIGINT NOT NULL AUTO_INCREMENT,
    `run_id`     BIGINT,
    `start_time` BIGINT,
    `time_delta` BIGINT,
    `pod_name`   VARCHAR(255),
    `cpu_usage` DOUBLE,
    `ram_usage` DOUBLE,
    `power_usage` DOUBLE,
    `interval`   BIGINT,
    KEY          `id` (`id`) USING BTREE
);
```