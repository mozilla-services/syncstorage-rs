FROM rust:1.54-buster as builder
WORKDIR /app
ADD . /app
ENV PATH=$PATH:/root/.cargo/bin
# temp removed --no-install-recommends due to CI docker build issue
RUN apt-get -q update && \
    apt-get -q install -y --no-install-recommends default-libmysqlclient-dev cmake golang-go python3-dev python3-pip python3-setuptools && \
    pip3 install -r requirements.txt && \
    rm -rf /var/lib/apt/lists/*

RUN \
    cargo --version && \
    rustc --version && \
    cargo install --path . --locked --root /app && \
    cargo install --path . --bin purge_ttl --locked --root /app

FROM debian:buster-slim
WORKDIR /app
COPY --from=builder /app/requirements.txt /app
RUN \
    groupadd --gid 10001 app && \
    useradd --uid 10001 --gid 10001 --home /app --create-home app && \
    apt-get -q update && \
    apt-get -q install -y build-essential default-libmysqlclient-dev libssl-dev ca-certificates libcurl4 python3-dev python3-pip python3-setuptools curl jq && \
    pip3 install -r /app/requirements.txt && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/bin /app/bin
COPY --from=builder /app/version.json /app
COPY --from=builder /app/spanner_config.ini /app
COPY --from=builder /app/tools/spanner /app/tools/spanner
COPY --from=builder /app/tools/integration_tests /app/tools/integration_tests
COPY --from=builder /app/scripts/prepare-spanner.sh /app/scripts/prepare-spanner.sh
COPY --from=builder /app/src/db/spanner/schema.ddl /app/schema.ddl

RUN chmod +x /app/scripts/prepare-spanner.sh

USER app:app

ENTRYPOINT ["/app/bin/syncstorage", "--config=spanner_config.ini"]
