#!/usr/bin/bash
cd `dirname $0`/../assets/

for d in libs css js; do
    cd $d

    rm -f *.zst *.br *.gz
    for f in *.js *.css; do
        zstd -19 -o $f.zst $f &
        gzip -9 -k $f &
    done

    cd ..
done

wait

ls -ahl libs/* css/*

