#!/usr/bin/env bash
#
# Bring up the Spanner emulator, provision the test database with the project
# schema, and run the syncstorage-rs Spanner unit tests.
#
# Idempotent: if the emulator is already up and the schema is in place, this
# just runs the tests. Re-running on a fresh database wipes nothing.
#
# Usage:
#   ./scripts/spanner-local-test-setup.sh                    # full flow
#   ./scripts/spanner-local-test-setup.sh setup              # bring up + provision, skip tests
#   ./scripts/spanner-local-test-setup.sh test               # run tests only (assumes setup is done)
#   ./scripts/spanner-local-test-setup.sh test <filter>      # run a single test by name
#   ./scripts/spanner-local-test-setup.sh down               # stop + remove emulator container
#
# Documented end-to-end in .claude/skills/spanner-local-tests/SKILL.md.

set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/.." && pwd)
cd "${REPO_ROOT}"

PROJECT_ID=test-project
INSTANCE_ID=test-instance
DATABASE_ID=test-database
REST_HOST=http://localhost:9020
GRPC_HOST=localhost:9010
COMPOSE_FILE="${REPO_ROOT}/docker/docker-compose.spanner.yaml"
SCHEMA_FILE="${REPO_ROOT}/syncstorage-spanner/src/schema.ddl"

DB_URL="spanner://projects/${PROJECT_ID}/instances/${INSTANCE_ID}/databases/${DATABASE_ID}"
DB_PATH="${REST_HOST}/v1/projects/${PROJECT_ID}/instances/${INSTANCE_ID}/databases/${DATABASE_ID}"

bold() { printf '\033[1m%s\033[0m\n' "$*"; }
note() { printf '  %s\n' "$*"; }

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "error: required command '$1' not found in PATH" >&2
        exit 1
    }
}

require_cmd docker
require_cmd curl
require_cmd jq
require_cmd cargo

start_emulator() {
    if curl -fsS -o /dev/null "${REST_HOST}/v1/projects/${PROJECT_ID}/instances" 2>/dev/null; then
        note "emulator already running"
        return 0
    fi
    bold "starting Spanner emulator"
    docker compose -f "${COMPOSE_FILE}" up -d sync-db >/dev/null
    for i in 1 2 3 4 5 6 7 8 9 10; do
        if curl -fsS -o /dev/null "${REST_HOST}/v1/projects/${PROJECT_ID}/instances" 2>/dev/null; then
            note "ready"
            return 0
        fi
        sleep 1
    done
    echo "error: emulator did not become ready within 10s" >&2
    exit 1
}

ensure_instance() {
    if curl -fsS -o /dev/null "${REST_HOST}/v1/projects/${PROJECT_ID}/instances/${INSTANCE_ID}" 2>/dev/null; then
        note "instance ${INSTANCE_ID} exists"
        return 0
    fi
    bold "creating instance ${INSTANCE_ID}"
    curl -fsS -X POST "${REST_HOST}/v1/projects/${PROJECT_ID}/instances" \
        -H 'Content-Type: application/json' \
        -d "{\"instance\":{\"config\":\"emulator-test-config\",\"nodeCount\":1,\"displayName\":\"Test\"},\"instanceId\":\"${INSTANCE_ID}\"}" \
        >/dev/null
}

ensure_database() {
    if curl -fsS -o /dev/null "${DB_PATH}" 2>/dev/null; then
        note "database ${DATABASE_ID} exists"
        return 0
    fi
    bold "creating database ${DATABASE_ID}"
    curl -fsS -X POST "${REST_HOST}/v1/projects/${PROJECT_ID}/instances/${INSTANCE_ID}/databases" \
        -H 'Content-Type: application/json' \
        -d "{\"createStatement\":\"CREATE DATABASE \`${DATABASE_ID}\`\"}" \
        >/dev/null
}

ensure_schema() {
    local count
    count=$(curl -fsS "${DB_PATH}/ddl" | jq -r '.statements | length')
    if [[ "${count}" -gt 0 ]]; then
        note "schema present (${count} statements)"
        return 0
    fi

    bold "applying schema from ${SCHEMA_FILE}"

    # macOS-safe parsing: BSD sed doesn't grok GNU sed's \+.
    # Strip comments, collapse whitespace, split on ;, drop empties.
    local stmts payload
    stmts=$(grep -v '^--' "${SCHEMA_FILE}" \
        | tr '\n' ' ' \
        | tr -s ' ' \
        | sed 's/;[[:space:]]*$//' \
        | jq -R -s -c 'split(";") | map(select(length > 0)) | map(gsub("^\\s+|\\s+$";""))')
    payload=$(jq -n --argjson stmts "${stmts}" '{statements:$stmts}')

    curl -fsS -X PATCH "${DB_PATH}/ddl" \
        -H 'Content-Type: application/json' \
        -d "${payload}" >/dev/null

    # The emulator processes DDL asynchronously. Wait for it to settle.
    for i in 1 2 3 4 5 6 7 8 9 10; do
        count=$(curl -fsS "${DB_PATH}/ddl" | jq -r '.statements | length')
        if [[ "${count}" -gt 0 ]]; then
            note "schema applied (${count} statements)"
            return 0
        fi
        sleep 1
    done
    echo "error: schema did not apply within 10s" >&2
    exit 1
}

run_tests() {
    local filter="${1:-}"
    bold "running spanner unit tests${filter:+ (filter: ${filter})}"
    SYNC_SYNCSTORAGE__SPANNER_EMULATOR_HOST="${GRPC_HOST}" \
    SYNC_SYNCSTORAGE__DATABASE_URL="${DB_URL}" \
    RUST_TEST_THREADS=1 \
    cargo test \
        --no-default-features \
        --features=syncstorage-db/spanner \
        --package syncstorage-db \
        ${filter:+"${filter}"} \
        ${filter:+-- --nocapture}
}

tear_down() {
    bold "stopping + removing sync-db container (other compose services untouched)"
    docker compose -f "${COMPOSE_FILE}" rm -fsv sync-db
}

setup_only() {
    start_emulator
    ensure_instance
    ensure_database
    ensure_schema
    note "ready: ${DB_URL}"
}

case "${1:-all}" in
    all)
        setup_only
        run_tests
        ;;
    setup)
        setup_only
        ;;
    test)
        run_tests "${2:-}"
        ;;
    down)
        tear_down
        ;;
    *)
        echo "usage: $0 [all|setup|test [<filter>]|down]" >&2
        exit 2
        ;;
esac
