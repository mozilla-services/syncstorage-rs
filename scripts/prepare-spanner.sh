#!/bin/sh

sleep 5

set -e

PROJECT_ID=test-project
INSTANCE_ID=test-instance
DATABASE_ID=test-database

DDL_STATEMENTS=$(
  grep -v ^-- schema.ddl \    # filter out comments
  | sed -n 's/ \+/ /gp' \     # trim two or more whitespace characters to one
  | tr -d '\n' \              # remove every newline
  | sed 's/\(.*\);/\1/' \     # remove the final semicolon
  | jq -R -s -c 'split(";")'  # split on semicolons and convert to JSON array
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
