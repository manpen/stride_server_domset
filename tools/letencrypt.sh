#!/bin/bash
set -e
set -x

cd `dirname $0`/..
. .env

if [ -z "$STRIDE_DOMAIN" ]; then
    echo "Error: STRIDE_DOMAIN is not set or is empty. Add it to the .env file."
    exit 1
fi

CERTBOT_COMMON_ARGS="--webroot -w assets --config-dir le --work-dir le --logs-dir le"
CERT_DIR="le/live/$STRIDE_DOMAIN"

if [ ! -d $CERT_DIR ]; then
    certbot certonly $CERTBOT_COMMON_ARGS -d $STRIDE_DOMAIN
else
    certbot renew $CERTBOT_COMMON_ARGS
fi

mkdir -p certs
cp $CERT_DIR/*.pem certs
