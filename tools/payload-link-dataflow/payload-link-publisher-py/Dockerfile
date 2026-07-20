# payload-link-publisher-py
#
# Dev/E2E-only Python variant of the payload-link Dataflow publisher.
# Prod continues to use the Java flex template one directory up. This
# image exists so the docker-compose e2e stack can run without a JVM
# or Dataflow.
#
# Build:
#   docker build -t payload-link-publisher-py:local \
#     tools/payload-link-dataflow/payload-link-publisher-py

FROM python:3.13-slim

RUN mkdir -p /app
ADD . /app
WORKDIR /app

RUN groupadd --gid 10001 app && \
    useradd --uid 10001 --gid 10001 --home /app -s /usr/sbin/nologin app

RUN \
    BUILD_DEPS="build-essential" && \
    apt-get update && \
    apt-get install -yq --no-install-recommends ${BUILD_DEPS} && \
    pip install --no-cache-dir poetry && \
    poetry config virtualenvs.create false && \
    poetry install --no-root --only main --no-interaction --no-ansi && \
    apt-get purge -yq --auto-remove ${BUILD_DEPS} && \
    apt-get autoremove -yqq && \
    apt-get clean -y && \
    rm -rf /var/lib/apt/lists/*

# Required env vars:
#   SPANNER_PROJECT_ID
#   SPANNER_INSTANCE_ID
#   SPANNER_DATABASE_ID
#   PUBSUB_PROJECT_ID
#   PUBSUB_TOPIC              - e.g. payload-link-changes
# Optional:
#   SPANNER_CHANGE_STREAM_NAME (default: payload_link_changes)
#   HEARTBEAT_MS               (default: 1000)
#   POLL_OVERLAP_SECONDS       (default: 5)
#   SPANNER_EMULATOR_HOST      (auto-honored by google-cloud-spanner)
#   PUBSUB_EMULATOR_HOST       (auto-honored by google-cloud-pubsub)
#   STATSD_HOST, STATSD_PORT

USER app:app
ENTRYPOINT ["python", "/app/publisher.py"]
