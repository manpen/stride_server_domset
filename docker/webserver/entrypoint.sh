#!/bin/bash
set -e
set -x


export DATABASE_URL=$DATABASE_URL_IN_DOCKER

. /root/.cargo/env

mkdir /target
export CARGO_TARGET_DIR="/target"

cd /srv
sqlx migrate run

while true; do
    cargo run --release
    sleep 5
done
