#!/usr/bin/env sh

if [ "${USE_HEAPTRACK:-false}" = "true" ]; then
    exec heaptrack "$BINARY" "$ARGS"
else
    exec "$BINARY" "$ARGS"
fi
