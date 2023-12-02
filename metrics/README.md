# Metrics Server

This is the component which collects all performance metrics from k8s clusters.

## Building & Running

### Building

### Configuring

### Running

## Development

### SQL Table

```
id (INT) | run_id (uuid) | start_time (INT) | time_delta (INT) | pod_name char(256) | cpu_usage (FLOAT) | ram_usage (FLOAT) | power_usage (FLOAT) | interval (INT) 
```