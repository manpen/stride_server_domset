#!/usr/bin/env bash
. ./testing.env

export DATABASE_URL=$DATABASE_URL
sqlx migrate run
#RUST_BACKTRACE=1 
RUST_LOG="debug" cargo test $@