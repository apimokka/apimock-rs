#!/bin/sh

cd crates
crates="apimock-routing apimock-config apimock-server"
for crate in $crates; do
    cd $crate
    cargo package
    cargo publish
    cd ..
done
cd ..

cargo package
cargo publish
