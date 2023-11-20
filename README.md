# Graph benchmarks
A set of tools to simplify benchmarking graph analytics platforms & graph databases

## Dependencies
* rust
* python
* ansible
* terraform
* vagrant (optional, if using vagrant as a platform)

## Getting started
1. Clone the repo
```
git clone https://github.com/graph-benchmarks/graph-benchmarks
```
Check the [example.config.toml](example.config.toml) for an example configuration for the benchmark runner.
2. Build everything
```
.build.sh
```
3. Run setup
> You will need to run setup with sudo when using the `vagrant` platform
```
bin/bench setup
```
4. Run benchmarks
```
bin/bench benchmark
```

## Config file
