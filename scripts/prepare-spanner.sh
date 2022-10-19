#!/bin/sh

#
# Copyright 2020 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
# This file contains modifications of the original version.
#

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
  "$SYNC_SYNCSTORAGE__SPANNER_EMULATOR_HOST/v1/projects/$PROJECT_ID/instances" \
  --header 'Accept: application/json' \
  --header 'Content-Type: application/json' \
  --data "{\"instance\":{\"config\":\"emulator-test-config\",\"nodeCount\":1,\"displayName\":\"Test Instance\"},\"instanceId\":\"$INSTANCE_ID\"}"

curl -sS --request POST \
  "$SYNC_SYNCSTORAGE__SPANNER_EMULATOR_HOST/v1/projects/$PROJECT_ID/instances/$INSTANCE_ID/databases" \
  --header 'Accept: application/json' \
  --header 'Content-Type: application/json' \
  --data "{\"createStatement\":\"CREATE DATABASE \`$DATABASE_ID\`\",\"extraStatements\":$DDL_STATEMENTS}"

sleep infinity
