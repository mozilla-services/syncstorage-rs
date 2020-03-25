FROM rust:1.40.0-buster as builder
WORKDIR /app
ADD . /app
ENV PATH=$PATH:/root/.cargo/bin
RUN apt-get -q update && \
    apt-get -q install -y --no-install-recommends default-libmysqlclient-dev cmake golang-go && \
    rm -rf /var/lib/apt/lists/* && \
    cd /app && \
    mkdir -m 755 bin

RUN \
    cargo --version && \
    rustc --version && \
    cargo install --path . --locked --root /app && \
    cargo install --path tools/spanner/purge_ttl --locked --root /app

FROM debian:buster-slim
WORKDIR /app
RUN \
    groupadd --gid 10001 app && \
    useradd --uid 10001 --gid 10001 --home /app --create-home app && \
    apt-get -q update && \
    apt-get -q install -y --no-install-recommends default-libmysqlclient-dev libssl-dev ca-certificates libcurl4 python3-venv python3-pip && \
    python3 -m pip install setuptools wheel && \
    python3 -m pip install google-cloud-spanner statsd && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/bin /app/bin
COPY --from=builder /app/version.json /app
COPY --from=builder /app/spanner_config.ini /app
COPY --from=builder /app/tools/spanner /app/tools/spanner

USER app:app

ENTRYPOINT ["/app/bin/syncstorage", "--config=spanner_config.ini"]
