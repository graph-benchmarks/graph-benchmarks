#!/bin/bash

cargo build --release || exit -1
mkdir -p bin || exit -1
cp target/release/benchmark-runner bin/bench || exit -1
echo Runner built!

drivers=(graphscope)

for i in "${drivers[@]}"
do
    pushd drivers/$i
    docker build -t $i . || exit -1
    echo $i image built!
done
