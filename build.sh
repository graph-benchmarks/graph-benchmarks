#!/bin/bash

python3 build-config.py

if [[ -z "${SKIP_CARGO}" ]]; then
    cargo build --release || exit -1
    mkdir -p bin || exit -1
    cp target/release/benchmark-runner bin/bench || exit -1
    echo Runner built!
else
    echo "Skipping cargo build"
fi