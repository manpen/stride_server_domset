#!/bin/bash
set -e
. ./.env

export MYSQL_PWD=$MYSQL_ROOT_PASSWORD
bzcat $1 | docker-compose exec -T mysql mysql -u root -p"$MYSQL_ROOT_PASSWORD" $MYSQL_DATABASE
    
    