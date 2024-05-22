# Graph benchmarks ![Visits](https://lambda.348575.xyz/repo-view-counter?repo=graph-benchmarks)
A tool to simplify benchmarking graph analytics platforms & graph databases

## Dependencies

* rust
* python
* ansible

## Install necessary providers:
* terraform
* vagrant also requires plugins: 
    * [libvirt](https://github.com/vagrant-libvirt/vagrant-libvirt)

## Getting started

1. Clone the repo

```
git clone https://github.com/graph-benchmarks/graph-benchmarks
```

Check the [example.config.toml](example.config.toml) for an example configuration for the benchmark runner, with all options explained

2. Build everything

```
./build.sh
```

### Options:
```
A graph benchmarking platform based on graphalytics

Usage: bench [OPTIONS] <COMMAND>

Commands:
  setup      Setup platform & kubernetes
  benchmark  Run benchmarks
  destroy    Teardown platform & kubernetes
  ls         List Resources
  dashboard  Port forward dashboard
  help       Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose      Verbose logging
  -f, --file <FILE>  Configuration file [default: config.toml]
  -h, --help         Print help
  -V, --version      Print version
```
> Use the environment variable `LOG` for debug logging, eg. `LOG=info`

3. Run setup
> You might need sudo/root access

```
bin/bench setup
```

4. List resources
```
bin/bench ls
```

5. Run benchmarks

```
bin/bench benchmark
```

6. Destroy resources
```
bin/bench destroy
```

## Datasets
Datasets are placed in the `datasets` folder, with a vertex and edge file.
eg:
```
├── twitter
│   ├── twitter.v
│   ├── twitter.e
│   ├── config.toml
├── facebook
│   ├── facebook.v
│   ├── facebook.e
│   ├── config.toml
```

The `config.toml` file contains these parameters:
```toml
weights = false
directed = false
```

## Graph platform arguments
Arguments to specific graph platforms can be provided in the config file and are driver specific.
```toml
[setup.graph_platform_args.graphscope]
cpu = 4
memory = "2Gi"
```

## Inbuilt providers

### Vagrant
Specify cpu and memory for nodes using vagrant
```toml
[setup.master_platform_env]
cpu = "8"
memory = "8192"

[setup.worker_platform_env]
cpu = "8"
memory = "8192"
```

### Terraform
Arguments to `[setup.worker_platform_env]` prefixed with `worker-{n}`, eg. `worker-0some_argument=3` to pass arguments to the object map used to spin up nodes

## Provider platform arguments
Arguments can be passed to platforms on a provider like shown below, for `vagrant-libvirt`
```toml
[setup.platform_args]
storage_pool_path = "/path/to/storage_pool"
```

## Drivers
Comes with drivers for:
* [graphscope](https://graphscope.io/)

## Changing build arguments
Build arguments are specified in [build.config.toml](build.config.toml). If you want to build and run a minimal setup, remove unnecessary drivers & platforms from the build config file, and re-run `build.sh`

## Writing drivers
Follow one of the reference implementations for a more detailed guide.

1. Create a rust library with the name `driver-config` inside the driver folder
2. Write your driver code
3. Write a dockerfile to containerize your driver
4. Add a `setup.yaml`, and a `remove.yaml` for creating and destroying the graph platforms
5. The `driver-config` rust crate must implement the `DriverConfig` trait present in `common/src/driver_config.rs`
6. Add it to `build.config.toml`

### Driver config file
The driver should take a single argument, to a yaml config file with the following format:
```yaml
config:
  id: 32 # integer database ID given for each run, provided by the runner
  algo: sssp # the algorithm to run
  log_file: /path/to/log/file

postgres:
  host: postgres host
  port: 5432
  db: postgres db
  user: postgres user
  ps: postgres password

platform:
  host: host of graph platform
  port: port of graph platform

dataset:
  vertex: /path/to/vertex/file
  edges: /path/to/edge/file
  name: dataset name
```

## Writing custom providers
1. Implement the `Platform` trait as a rust crate present in `common/src/provider.rs`
