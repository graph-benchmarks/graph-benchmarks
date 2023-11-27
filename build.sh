#!/bin/bash

cargo build --release || exit -1
mkdir -p bin || exit -1
cp target/release/benchmark-runner bin/bench || exit -1
echo Runner built!

while IFS= read -r line || [[ -n "$line" ]]; do
    pushd drivers/$line
    docker build -t $line . || exit -1
    echo $line image built!
    popd
    docker save $line -o bin/$line.tar || exit -1
done < build-drivers