#!/bin/bash
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# Post-run report for the offload load-test rig (docker-compose.loadtest.yaml).
#
# Molotov tells you what the client saw. This tells you what the pipeline did
# with it: whether the change stream -> publisher -> Pub/Sub -> reconciler arm
# drained, and whether every offloaded object ended up finalized.
#
# Run it with the stack still up, after the loadtest container has exited:
#
#   make loadtest-offload-report
#   docker/loadtest-report.sh --no-wait      # skip the drain wait
#
# Requires curl and jq on the host. Reads the emulators through their
# published ports, so it needs no docker exec.

set -uo pipefail

STATSD_URL="${STATSD_URL:-http://localhost:9102/metrics}"
GCS_HOST="${GCS_HOST:-http://localhost:4443}"
GCS_PAYLOAD_BUCKET="${GCS_PAYLOAD_BUCKET:-test-payloads}"

# Drain wait: poll until the reconciler's handled-record total stops moving for
# STABLE_POLLS consecutive polls, or DRAIN_TIMEOUT seconds elapse.
POLL_INTERVAL="${POLL_INTERVAL:-5}"
STABLE_POLLS="${STABLE_POLLS:-3}"
DRAIN_TIMEOUT="${DRAIN_TIMEOUT:-300}"

WAIT_FOR_DRAIN=1
[ "${1:-}" = "--no-wait" ] && WAIT_FOR_DRAIN=0

# Set to 1 by any failed health assertion; becomes the exit code.
problems=0

for tool in curl jq; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "error: $tool is required but not installed" >&2
        exit 2
    fi
done

hr() { printf '%s\n' "----------------------------------------------------------------"; }

# Scrape the exporter once.
scrape() { curl -sf --max-time 10 "$STATSD_URL"; }

# Sum every sample of a metric family, across all label combinations.
# statsd counters arrive as `name{labels} value`; a bare counter has no braces.
# Prints 0 when the family has not been emitted yet.
metric_sum() {
    local body="$1" name="$2"
    printf '%s\n' "$body" \
        | awk -v n="$name" '
            $0 ~ "^"n"($|[{ ])" {
                v = $NF
                if (v + 0 == v) s += v
            }
            END { printf "%d\n", s + 0 }
        '
}

# Print every sample line for a metric prefix, minus the HELP/TYPE comments.
metric_lines() {
    local body="$1" prefix="$2"
    printf '%s\n' "$body" | grep -E "^${prefix}" | grep -v '^#' | sort
}

if ! body="$(scrape)"; then
    echo "error: cannot reach the statsd exporter at $STATSD_URL" >&2
    echo "       is the stack up? (make loadtest-offload-up)" >&2
    exit 2
fi

# ---------------------------------------------------------------- drain wait

# Total records the reconciler has finished with, by any route. While this is
# still climbing there is backlog in Pub/Sub or the publisher.
handled_total() {
    local b="$1" sum=0 m
    for m in payload_reconciler_finalizes \
             payload_reconciler_orphan_deletes \
             payload_reconciler_noop_skips \
             payload_reconciler_batch_commit_skips \
             payload_reconciler_gcs_404; do
        sum=$(( sum + $(metric_sum "$b" "$m") ))
    done
    printf '%d\n' "$sum"
}

if [ "$WAIT_FOR_DRAIN" -eq 1 ]; then
    echo "waiting for the reconciler to drain (timeout ${DRAIN_TIMEOUT}s)..."
    last=-1
    stable=0
    elapsed=0
    while [ "$elapsed" -lt "$DRAIN_TIMEOUT" ]; do
        current="$(handled_total "$body")"
        if [ "$current" -eq "$last" ]; then
            stable=$(( stable + 1 ))
        else
            stable=0
        fi
        printf '  handled=%s stable=%s/%s\n' "$current" "$stable" "$STABLE_POLLS"
        if [ "$stable" -ge "$STABLE_POLLS" ]; then
            if [ "$current" -gt 0 ]; then
                echo "  drained."
            else
                # Flat at zero is not a drained pipeline, it is a pipeline that
                # never started. Bail out now instead of burning the full
                # timeout waiting for a number that will not move.
                echo "  no progress at all -- nothing reached the reconciler."
                problems=1
            fi
            break
        fi
        last="$current"
        sleep "$POLL_INTERVAL"
        elapsed=$(( elapsed + POLL_INTERVAL ))
        body="$(scrape)" || { echo "  scrape failed, retrying" >&2; continue; }
    done
    if [ "$elapsed" -ge "$DRAIN_TIMEOUT" ]; then
        echo "  warning: still moving at timeout -- the numbers below are a" \
             "snapshot of an undrained pipeline."
        problems=1
    fi
fi

# ------------------------------------------------------------------- metrics

hr
echo "RECONCILER (payload_reconciler.*)"
hr
lines="$(metric_lines "$body" 'payload_reconciler_')"
if [ -z "$lines" ]; then
    echo "  no metrics emitted -- the reconciler never handled a record."
    echo "  check: docker compose logs payload-link-publisher payload-reconciler"
    problems=1
else
    printf '%s\n' "$lines" | sed 's/^/  /'
fi

finalizes="$(metric_sum "$body" payload_reconciler_finalizes)"
orphan_deletes="$(metric_sum "$body" payload_reconciler_orphan_deletes)"
handler_errors="$(metric_sum "$body" payload_reconciler_errors)"
noop_skips="$(metric_sum "$body" payload_reconciler_noop_skips)"
batch_skips="$(metric_sum "$body" payload_reconciler_batch_commit_skips)"

hr
echo "PUBLISHER (payload_link_publisher.*)"
hr
lines="$(metric_lines "$body" 'payload_link_publisher_')"
if [ -z "$lines" ]; then
    echo "  no metrics emitted -- the publisher never read a change record."
    echo "  check: docker compose logs payload-link-publisher"
    problems=1
else
    printf '%s\n' "$lines" | sed 's/^/  /'
fi

published="$(metric_sum "$body" payload_link_publisher_published)"
filtered="$(metric_sum "$body" payload_link_publisher_filtered)"
pub_errors="$(metric_sum "$body" payload_link_publisher_errors)"

hr
echo "SYNCSERVER (syncstorage.*)"
hr
lines="$(metric_lines "$body" 'syncstorage_')"
if [ -z "$lines" ]; then
    echo "  no metrics emitted."
    problems=1
else
    printf '%s\n' "$lines" | sed 's/^/  /'
fi

# --------------------------------------------------------------- GCS objects

hr
echo "GCS OBJECTS (${GCS_PAYLOAD_BUCKET})"
hr

# Page through the bucket. fake-gcs honours maxResults/pageToken like the real
# JSON API, and a load test can easily leave tens of thousands of objects.
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
token=""
pages=0
while :; do
    url="${GCS_HOST}/storage/v1/b/${GCS_PAYLOAD_BUCKET}/o?maxResults=1000"
    [ -n "$token" ] && url="${url}&pageToken=${token}"
    if ! page="$(curl -sf --max-time 30 "$url")"; then
        echo "  error: cannot list ${GCS_PAYLOAD_BUCKET} at ${GCS_HOST}" >&2
        problems=1
        break
    fi
    printf '%s' "$page" | jq -c '.items // [] | .[]' >>"$tmp"
    token="$(printf '%s' "$page" | jq -r '.nextPageToken // empty')"
    pages=$(( pages + 1 ))
    [ -z "$token" ] && break
done

total="$(wc -l <"$tmp" | tr -d ' ')"
if [ "$total" = "0" ]; then
    echo "  bucket is empty -- no payload was offloaded."
    echo "  check that the collections molotov writes to overlap"
    echo "  SYNC_SYNCSTORAGE__GCS_PAYLOAD_OFFLOAD_COLLECTIONS."
    problems=1
else
    committed="$(jq -rs '[.[] | select(.metadata.committed == "true")] | length' <"$tmp")"
    uncommitted=$(( total - committed ))
    # Finalize also pins customTime to the far-future sentinel; that is what
    # actually protects the object from the 30-day lifecycle rule, so it is
    # worth counting separately from the metadata flag.
    pinned="$(jq -rs '[.[] | select((.customTime // "") | startswith("2200-12-31"))] | length' <"$tmp")"
    bytes="$(jq -rs '[.[] | (.size // "0" | tonumber)] | add // 0' <"$tmp")"

    printf '  objects           %s (across %s list page(s))\n' "$total" "$pages"
    printf '  committed=true    %s\n' "$committed"
    printf '  committed!=true   %s\n' "$uncommitted"
    printf '  customTime pinned %s\n' "$pinned"
    printf '  total bytes       %s (%s MiB)\n' "$bytes" "$(( bytes / 1048576 ))"

    # A large tail of unfinalized objects is expected, not a fault, and the
    # expected size is derivable from the load test's own distributions.
    #
    # loadtest.py picks num_requests from post_count_distribution
    # ([67,18,9,4,2] -> P(0)=.67, P(1)=.18, P(>=2)=.15) and flips a coin for
    # `transact`. A transactional run with exactly one request opens the batch
    # with ?batch=true and the loop ends before it ever commits, so those
    # uploads correctly stay committed=false until the lifecycle rule reaps
    # them. Of the scenarios that write at all (P=.33), the never-committed
    # share is .5 * .18/.33, about 28%. Measured runs land right on that.
    #
    # So the threshold has to sit comfortably above ~28%, not below it. Only a
    # share far past that means the reconciler is genuinely not keeping up.
    if [ "$uncommitted" -gt 0 ]; then
        share=$(( uncommitted * 100 / total ))
        printf '  unfinalized share %s%% (about 28%% is expected, see comment)\n' "$share"
        if [ "$share" -ge "${UNFINALIZED_FAIL_PCT:-50}" ]; then
            echo "  FAIL: $uncommitted of $total objects never finalized"
            echo "        (>= ${UNFINALIZED_FAIL_PCT:-50}%). Either the reconciler is not"
            echo "        keeping up with the write rate, or finalize is broken."
            problems=1
        else
            echo "  NOTE: $uncommitted object(s) unfinalized. At this share it is the"
            echo "        expected tail: molotov opens batches it never commits, and"
            echo "        those uploads stay committed=false by design."
        fi
    fi
    if [ "$committed" != "$pinned" ]; then
        echo "  warning: committed count and pinned-customTime count disagree."
        echo "  Finalize sets both in one patch, so they should match."
        problems=1
    fi
fi

# -------------------------------------------------------------------- verdict

hr
echo "VERDICT"
hr
printf '  published            %s\n' "$published"
printf '  filtered (inert)     %s\n' "$filtered"
printf '  publisher errors     %s\n' "$pub_errors"
printf '  finalizes            %s\n' "$finalizes"
printf '  orphan_deletes       %s\n' "$orphan_deletes"
printf '  batch_commit_skips   %s\n' "$batch_skips"
printf '  noop_skips           %s\n' "$noop_skips"
printf '  handler errors       %s\n' "$handler_errors"

if [ "$pub_errors" -gt 0 ]; then
    echo "  FAIL: publisher errors are non-zero. Change records were either"
    echo "        not read or not published; the reconciler numbers below are"
    echo "        an undercount."
    echo "        See: docker compose logs payload-link-publisher"
    problems=1
fi
# The positive form of the STOR-628 claim. noop_skips == 0 only says the
# reconciler saw nothing inert; a non-zero filtered count says the publisher
# actively dropped inert records, which is what the filter is for. On a mixed
# run there must be some: every inline write and every meta/clients write
# produces a change record with payload_link NULL on both sides.
if [ "$filtered" -eq 0 ]; then
    echo "  WARN: nothing was filtered. On a mixed run this means either the"
    echo "        filter is passing inert records through, or every write was"
    echo "        offloaded and the NULL-filtering path went untested."
    problems=1
fi
if [ "$handler_errors" -gt 0 ]; then
    echo "  FAIL: handler errors are non-zero. Messages were left unacked;"
    echo "        five failures on one message routes it to the DLQ in prod."
    echo "        See: docker compose logs payload-reconciler"
    problems=1
fi
if [ "$noop_skips" -gt 0 ]; then
    echo "  FAIL: noop_skips is non-zero. The publisher's filter let records"
    echo "        through that had payload_link NULL on both sides -- the"
    echo "        exact regression STOR-628 is meant to catch."
    problems=1
fi
if [ "$finalizes" -eq 0 ] && [ "$total" != "0" ]; then
    echo "  FAIL: objects exist but nothing was finalized. The change stream,"
    echo "        publisher or Pub/Sub leg is broken."
    problems=1
fi

# The Spanner emulator emits transaction_tag on change records but never
# populates it, so the reconciler cannot recognise a batch-commit handoff and
# deletes the object the just-committed bsos row points at. That produces
# batch_commit_skips == 0 alongside a high gcs_404{op=finalize}. It is a known
# emulator gap rather than a regression, so it is reported and not counted
# against the verdict by default. On GCP dev, where Spanner does record the
# tag, this must be treated as a hard failure: run with STRICT_BATCH_COMMIT=1.
finalize_404="$(printf '%s\n' "$body" \
    | awk '/^payload_reconciler_gcs_404\{[^}]*op="finalize"/ { s += $NF } END { printf "%d\n", s + 0 }')"
if [ "$batch_skips" -eq 0 ] && [ "$finalize_404" -gt 0 ]; then
    if [ "${STRICT_BATCH_COMMIT:-0}" = "1" ]; then
        echo "  FAIL: batch_commit_skips is 0 with $finalize_404 finalize 404s."
        echo "        The batch_commit transaction tag is not reaching the"
        echo "        reconciler, so batch-commit handoff objects are being"
        echo "        deleted (STOR-657/668). Strict mode: treated as failure."
        problems=1
    else
        echo "  NOTE: batch_commit_skips is 0 with $finalize_404 finalize 404s."
        echo "        Expected on the local emulator, which never populates"
        echo "        transaction_tag, so every batch-commit handoff looks like"
        echo "        a genuine removal. See the emulator section in"
        echo "        docs/src/tools/load-testing.md, and reproduce in isolation"
        echo "        with docker/batch-commit-probe.py."
        echo "        On GCP dev this is a real failure: re-run with"
        echo "        STRICT_BATCH_COMMIT=1."
    fi
fi

if [ "$problems" -eq 0 ]; then
    echo "  OK: pipeline drained clean."
fi
exit "$problems"
