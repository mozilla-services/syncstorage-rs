#!/usr/bin/env bash
# Produces a unified diff of the *complete* delta between upstream's
# Cloud_Spanner_Change_Streams_to_PubSub template (at the SHA pinned
# in upstream.txt) and our impl. Reference / review artifact only --
# not a build input, not called by CI, and not consumed by anything
# else in this repo.
#
# The checked-in upstream-customization.patch expresses only the
# filter step, which is what a reviewer usually wants to see. This
# script exists for the case where a reviewer wants the full picture
# (sink swap, dropped options, JSON shape, etc. -- see the "Beyond
# the filter" section in README.md).
#
# Output: unified diff on stdout. Exit 1 = differences exist (normal),
# exit 0 = files identical, exit >1 = error.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOCAL_IMPL="$SCRIPT_DIR/src/main/java/com/mozilla/sync/payloadlink/PayloadLinkChangesToPubSub.java"
UPSTREAM_PATH="v2/googlecloud-to-googlecloud/src/main/java/com/google/cloud/teleport/v2/templates/SpannerChangeStreamsToPubSub.java"
UPSTREAM_REPO="https://github.com/GoogleCloudPlatform/DataflowTemplates.git"

if [[ ! -f "$LOCAL_IMPL" ]]; then
    echo "generate-full-delta.sh: cannot find local impl at $LOCAL_IMPL" >&2
    exit 2
fi
if [[ ! -f "$SCRIPT_DIR/upstream.txt" ]]; then
    echo "generate-full-delta.sh: cannot find upstream.txt" >&2
    exit 2
fi

SHA="$(tr -d '[:space:]' < "$SCRIPT_DIR/upstream.txt")"
if [[ -z "$SHA" ]]; then
    echo "generate-full-delta.sh: upstream.txt is empty" >&2
    exit 2
fi

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

# Fetch only the file we need at the pinned SHA. Sparse-checkout keeps
# the working tree small; the deepen fetch lets `git checkout <sha> --`
# resolve if the SHA is older than the initial shallow tip.
(
    cd "$TMPDIR"
    git clone --quiet --depth 1 --filter=blob:none --sparse "$UPSTREAM_REPO" dt >/dev/null 2>&1
    cd dt
    git sparse-checkout set --no-cone "/$UPSTREAM_PATH" >/dev/null 2>&1
    # Some SHAs are behind the depth-1 tip; deepen until git can resolve.
    git fetch --quiet --depth 200 origin main >/dev/null 2>&1 || true
    git checkout --quiet "$SHA" -- "$UPSTREAM_PATH"
) || {
    echo "generate-full-delta.sh: failed to fetch upstream $SHA" >&2
    exit 2
}

UPSTREAM_FILE="$TMPDIR/dt/$UPSTREAM_PATH"
TRANSFORMED="$TMPDIR/mod.java"

# Textually transform our impl to sit at upstream's package/class name
# so the diff shows *behaviour* differences, not cosmetic renames.
# If our impl ever adds a new options-method that also differs in
# upstream's SpannerChangeStreamsToPubSubOptions naming, extend the
# sed pipeline below.
sed \
    -e 's|^package com\.mozilla\.sync\.payloadlink;|package com.google.cloud.teleport.v2.templates;|' \
    -e 's|^import com\.mozilla\.sync\.payloadlink\.PayloadLinkOptions;|import com.google.cloud.teleport.v2.options.SpannerChangeStreamsToPubSubOptions;|' \
    -e 's|PayloadLinkOptions|SpannerChangeStreamsToPubSubOptions|g' \
    -e 's|PayloadLinkChangesToPubSub|SpannerChangeStreamsToPubSub|g' \
    -e 's|options\.getChangeStreamName()|options.getSpannerChangeStreamName()|g' \
    "$LOCAL_IMPL" > "$TRANSFORMED"

# `diff -b` ignores whitespace-amount differences (our impl uses
# 4-space indent, upstream uses 2-space; without -b every method line
# reads as changed even when semantically identical). `diff` returns
# 1 when files differ (expected); shield that from the outer `set -e`.
diff -u -b \
    --label "a/$UPSTREAM_PATH" \
    --label "b/$UPSTREAM_PATH" \
    "$UPSTREAM_FILE" "$TRANSFORMED" \
    && rc=0 || rc=$?

# 0 = identical (surprising), 1 = differ (normal), >=2 = error.
if [[ $rc -gt 1 ]]; then
    echo "generate-full-delta.sh: diff failed with exit $rc" >&2
    exit "$rc"
fi
exit "$rc"
