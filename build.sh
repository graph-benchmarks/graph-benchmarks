#!/bin/bash

cargo build --release || exit -1
mkdir -p bin || exit -1
cp target/release/benchmark-runner bin/bench || exit -1
echo Runner built!

while IFS= read -r line || [[ -n "$line" ]]; do
    pushd drivers/$i
    docker build -t $i . || exit -1
    echo $i image built!
    popd
    docker save $i -o bin/$i.tar || exit -1
done < build-drivers