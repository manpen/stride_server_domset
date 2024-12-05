#!/bin/bash
set -e
set -x

export DATABASE_URL=$DATABASE_URL_IN_DOCKER

. /root/.cargo/env

mkdir -p /target
export CARGO_TARGET_DIR="/target"

cd /srv

mkdir -p /active

# build and execute the server
cargo build --bin server --release $STRIDE_BUILD
mv /target/release/server /active/server
rm -rf /target

cd /srv

while true; do
    /active/server 
    sleep 0.5
done
