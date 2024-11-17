#!/bin/bash
set -e
docker-compose exec mysql /bin/bash -c \
    'MYSQL_PWD=$MYSQL_ROOT_PASSWORD mysqldump -u root $MYSQL_DATABASE' \
    | bzip2 db_backup.sql.bz2
    
    