#!/bin/sh

sleep 5

set -e

PROJECT_ID=test-project
INSTANCE_ID=test-instance
DATABASE_ID=test-database
DDL_STATEMENTS=$(jq -R -s -c 'split("\n")' < schema.ddl)

curl -sS --request POST \
  "db:9020/v1/projects/$PROJECT_ID/instances" \
  --header 'Accept: application/json' \
  --header 'Content-Type: application/json' \
  --data "{\"instance\":{\"config\":\"emulator-test-config\",\"nodeCount\":1,\"displayName\":\"Test Instance\"},\"instanceId\":\"$INSTANCE_ID\"}"

curl -sS --request POST \
  "db:9020/v1/projects/$PROJECT_ID/instances/$INSTANCE_ID/databases" \
  --header 'Accept: application/json' \
  --header 'Content-Type: application/json' \
  --data "{\"createStatement\":\"CREATE DATABASE \`$DATABASE_ID\`\",\"extraStatements\":$DDL_STATEMENTS}"

sleep infinity