#!/usr/bin/env bash
docker-compose -f docker-compose-testing.yml up -d
. ./testing.env
sqlx migrate run
cargo test