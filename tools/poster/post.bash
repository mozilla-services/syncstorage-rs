#!/bin/bash
NODE="http://localhost:8000"
URI="/1.5/1/storage/col2/DEADBEEF"
AUTH=`../hawk/venv/bin/python ../hawk/make_hawk_token.py --node $NODE --uri $URI --as_header`
curl -vv -X PUT "$NODE$URI" \
    -H "Authorization: $AUTH" \
    -H 'Content-Type: application/json' \
    -H 'Accept: application/json' \
    -d '{"id": "womble", "payload": "mary had a little lamb with a nice mint jelly", "sortindex": 0, "ttl": 86400}'
