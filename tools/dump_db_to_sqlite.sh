#!/bin/bash
set -x
set -e

PYENV="pyenv"
SQLITE_DB_FILE="db_meta.db"

. ../.env

if [ ! -d $PYENV ]; then
    python3 -m venv $PYENV
    . $PYENV/bin/activate
    $PYENV/bin/pip3 install mysql-to-sqlite3
fi

. $PYENV/bin/activate

rm -rf ${SQLITE_DB_FILE}

mysql2sqlite \
    -f $SQLITE_DB_FILE \
    -d $MYSQL_DATABASE \
    -u $MYSQL_USER \
    --mysql-password $MYSQL_PASSWORD \
    -h $MYSQL_HOST \
    -P $MYSQL_PORT \
    -e InstanceData _sqlx_migrations Solution SolutionData 

gzip -k $SQLITE_DB_FILE
mv -f $SQL_DB_FILE $SQLITE_DB_FILE.gz ../assets/

