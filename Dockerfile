FROM rust:1.46-buster as builder
WORKDIR /app
ADD . /app
ENV PATH=$PATH:/root/.cargo/bin
# temp removed --no-install-recommends due to CI docker build issue
RUN apt-get -q update && \
    apt-get -q install -y --no-install-recommends default-libmysqlclient-dev cmake golang-go python3-dev python3-pip && \
    pip3 install tokenlib && \
    rm -rf /var/lib/apt/lists/* && \
    cd /app && \
    mkdir -m 755 bin

RUN \
    cargo --version && \
    rustc --version && \
    cargo install --path . --locked --root /app && \
    cargo install --path . --bin purge_ttl --locked --root /app

FROM debian:buster-slim
WORKDIR /app
RUN \
    groupadd --gid 10001 app && \
    useradd --uid 10001 --gid 10001 --home /app --create-home app && \
    apt-get -q update && \
    apt-get -q install -y build-essential default-libmysqlclient-dev libssl-dev ca-certificates libcurl4 python3-dev python3-pip && \
    pip3 install tokenlib && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/bin /app/bin
COPY --from=builder /app/version.json /app
COPY --from=builder /app/spanner_config.ini /app
COPY --from=builder /app/tools/spanner /app/tools/spanner
COPY --from=builder /app/tools/integration_tests /app/tools/integration_tests
RUN mkdir -p /app/project && chmod 755 -R /app && chown -R app /

USER app:app

ENTRYPOINT ["/app/bin/syncstorage", "--config=spanner_config.ini"]
