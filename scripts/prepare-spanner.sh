#!/bin/sh

sleep 5

set -e

PROJECT_ID=test-project
INSTANCE_ID=test-instance
DATABASE_ID=test-database

DDL_STATEMENTS=$(
  grep -v ^-- schema.ddl \
  | sed -n 's/ \+/ /gp' \
  | tr -d '\n' \
  | sed 's/\(.*\);/\1/' \
  | jq -R -s -c 'split(";")'
) 

curl -sS --request POST \
  "$SYNC_SPANNER_EMULATOR_HOST/v1/projects/$PROJECT_ID/instances" \
  --header 'Accept: application/json' \
  --header 'Content-Type: application/json' \
  --data "{\"instance\":{\"config\":\"emulator-test-config\",\"nodeCount\":1,\"displayName\":\"Test Instance\"},\"instanceId\":\"$INSTANCE_ID\"}"

curl -sS --request POST \
  "$SYNC_SPANNER_EMULATOR_HOST/v1/projects/$PROJECT_ID/instances/$INSTANCE_ID/databases" \
  --header 'Accept: application/json' \
  --header 'Content-Type: application/json' \
  --data "{\"createStatement\":\"CREATE DATABASE \`$DATABASE_ID\`\",\"extraStatements\":$DDL_STATEMENTS}"

sleep infinity
