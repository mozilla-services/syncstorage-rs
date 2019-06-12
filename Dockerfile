FROM rust:1.35.0-stretch as builder
WORKDIR /app
ADD . /app
ENV PATH=$PATH:/root/.cargo/bin
RUN apt-get -q update && \
    apt-get -q install -y default-libmysqlclient-dev && \
    cd /app && \
    mkdir -m 755 bin

RUN \
    cargo --version && \
    rustc --version && \
    cargo build && \
    cp target/debug/syncstorage bin

FROM debian:stretch-slim
WORKDIR /app
RUN \
    groupadd --gid 10001 app && \
    useradd --uid 10001 --gid 10001 --home /app --create-home app && \
    apt-get -q update && \
    apt-get -q install -y default-libmysqlclient-dev libssl-dev ca-certificates && \
    rm -rf /var/lib/apt/lists

COPY --from=builder /app/bin /app/bin
COPY --from=builder /app/version.json /app

CMD ["/app/bin/syncstorage"]
