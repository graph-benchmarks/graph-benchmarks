#!/bin/bash

cargo build --release
mkdir -p bin
cp target/release/benchmark-runner bin/bench