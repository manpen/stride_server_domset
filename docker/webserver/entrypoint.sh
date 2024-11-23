#!/bin/bash
set -e
set -x


export DATABASE_URL=$DATABASE_URL_IN_DOCKER

. /root/.cargo/env

mkdir -p /target
export CARGO_TARGET_DIR="/target"

cd /srv
sqlx migrate run

while true; do
    RUST_LOG="debug" cargo run --bin server --release
    sleep 0.5
done
