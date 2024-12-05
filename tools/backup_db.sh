#!/bin/bash
set -e
docker-compose exec mysql /bin/bash -c \
    'MYSQL_PWD=$MYSQL_ROOT_PASSWORD mysqldump -u root $MYSQL_DATABASE --add-drop-table --ignore-table $MYSQL_DATABASE._sqlx_migrations' \
    | gzip -c > db_backup.sql.gz
    
    