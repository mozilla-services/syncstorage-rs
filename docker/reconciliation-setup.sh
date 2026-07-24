#!/bin/sh
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# One-shot init for the reconciliation e2e compose stack.
#
# Creates:
#   * Pub/Sub topic         : payload-link-changes
#   * Pub/Sub subscription  : payload-link-reconciler-sub
#   * GCS bucket            : $GCS_PAYLOAD_BUCKET (fake-gcs-server)
#
# All resources are keyed on $PROJECT_ID for both emulators. Modeled on
# scripts/prepare-spanner.sh -- curl the emulator REST APIs, no gcloud
# dependency.

set -e

PROJECT_ID="${PROJECT_ID:-test-project}"
PUBSUB_HOST="${PUBSUB_HOST:-http://pubsub-emulator:8085}"
GCS_HOST="${GCS_HOST:-http://fake-gcs:4443}"
TOPIC="${PUBSUB_TOPIC:-payload-link-changes}"
SUBSCRIPTION="${PUBSUB_SUBSCRIPTION:-payload-link-reconciler-sub}"
BUCKET="${GCS_PAYLOAD_BUCKET:-test-payloads}"

echo "reconciliation-setup: waiting for emulators..."

# Small wait; the depends_on / healthcheck in compose is the real gate.
sleep 3

echo "reconciliation-setup: creating Pub/Sub topic ${PROJECT_ID}/${TOPIC}"
curl -sS -X PUT \
  "${PUBSUB_HOST}/v1/projects/${PROJECT_ID}/topics/${TOPIC}" \
  --header 'Content-Type: application/json' \
  --data '{}' >/dev/null

echo "reconciliation-setup: creating Pub/Sub subscription ${PROJECT_ID}/${SUBSCRIPTION}"
curl -sS -X PUT \
  "${PUBSUB_HOST}/v1/projects/${PROJECT_ID}/subscriptions/${SUBSCRIPTION}" \
  --header 'Content-Type: application/json' \
  --data "{\"topic\":\"projects/${PROJECT_ID}/topics/${TOPIC}\",\"ackDeadlineSeconds\":60}" >/dev/null

echo "reconciliation-setup: creating GCS bucket ${BUCKET}"
curl -sS -X POST \
  "${GCS_HOST}/storage/v1/b?project=${PROJECT_ID}" \
  --header 'Content-Type: application/json' \
  --data "{\"name\":\"${BUCKET}\"}" >/dev/null

echo "reconciliation-setup: done"
