#!/usr/bin/env bash
docker-compose -f docker-compose-testing.yml up -d
. ./testing.env
sqlx migrate run
RUST_BACKTRACE=1 RUST_LOG="debug" cargo test $@