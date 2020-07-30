#!/bin/bash
NODE="http://localhost:8000"
URI="/1.5/1/storage/col2/DEADBEEF"
METHOD="PUT"
SYNC_MASTER_SECRET="INSERT_SECRET_KEY_HERE"
AUTH=`../hawk/venv/bin/python ../hawk/make_hawk_token.py --node $NODE --uri $URI --method $METHOD --secret=$SYNC_MASTER_SECRET --as_header`
curl -vv -X PUT "$NODE$URI" \
    -H "$AUTH" \
    -H 'Content-Type: application/json' \
    -H 'Accept: application/json' \
    -d '{"id": "womble", "payload": "mary had a little lamb with a nice mint jelly", "sortindex": 0, "ttl": 86400}'
