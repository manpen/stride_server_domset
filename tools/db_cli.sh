#!/bin/bash
set -e
. ./.env

export MYSQL_PWD=$MYSQL_ROOT_PASSWORD
docker-compose exec mysql mysql -u root -p"$MYSQL_ROOT_PASSWORD" $MYSQL_DATABASE
    
    