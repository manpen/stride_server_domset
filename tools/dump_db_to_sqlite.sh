#!/bin/bash
set -e

PYENV="pyenv"
SQLITE_DB_FILE="runner.db"
SQLITE_DATA_FILE="smalldata.db"

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
    -e InstanceData _sqlx_migrations 

echo "UPDATE solutiondata SET data = NULL;" \
    | sqlite3 $SQLITE_DB_FILE


mysql2sqlite \
    -f $SQLITE_DATA_FILE \
    -d $MYSQL_DATABASE \
    -u $MYSQL_USER \
    --mysql-password $MYSQL_PASSWORD \
    -h $MYSQL_HOST \
    -P $MYSQL_PORT \
    -t InstanceData

echo "DELETE FROM instancedata WHERE length(data) > 1000;" \
    | sqlite3 $SQLITE_DATA_FILE


for f in $SQLITE_DATA_FILE $SQLITE_DB_FILE; do
    echo "VACUUM;" | sqlite3 $f    
    mv $f ../assets/
done