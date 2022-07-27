FROM rust:1.60-buster as builder
WORKDIR /app
ADD . /app
ENV PATH=$PATH:/root/.cargo/bin
# temp removed --no-install-recommends due to CI docker build issue
RUN apt-get -q update && \
    apt-get -q install -y --no-install-recommends default-libmysqlclient-dev cmake golang-go python3-dev python3-pip python3-setuptools python3-wheel && \
    pip3 install -r requirements.txt && \
    rm -rf /var/lib/apt/lists/*

RUN \
    cargo --version && \
    rustc --version && \
    cargo install --path ./syncstorage --locked --root /app && \
    cargo install --path ./syncstorage --locked --root /app --bin purge_ttl

FROM debian:buster-slim
WORKDIR /app
COPY --from=builder /app/requirements.txt /app
# Due to a build error that occurs with the Python cryptography package, we
# have to set this env var to prevent the cryptography package from building
# with Rust. See this link for more information:
# https://pythonshowcase.com/question/problem-installing-cryptography-on-raspberry-pi
ENV CRYPTOGRAPHY_DONT_BUILD_RUST=1
RUN \
    groupadd --gid 10001 app && \
    useradd --uid 10001 --gid 10001 --home /app --create-home app && \
    apt-get -q update && \
    apt-get -q install -y build-essential default-libmysqlclient-dev libssl-dev libffi-dev ca-certificates libcurl4 python3-dev python3-pip python3-setuptools python3-wheel cargo curl jq && \
    # The python3-cryptography debian package installs version 2.6.1, but we
    # we want to use the version specified in requirements.txt. To do this,
    # we have to remove the python3-cryptography package here.
    apt-get -q remove -y python3-cryptography && \
    pip3 install -r /app/requirements.txt && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/bin /app/bin
COPY --from=builder /app/syncstorage/version.json /app
COPY --from=builder /app/spanner_config.ini /app
COPY --from=builder /app/tools/spanner /app/tools/spanner
COPY --from=builder /app/tools/integration_tests /app/tools/integration_tests
COPY --from=builder /app/tools/tokenserver/process_account_events.py /app/tools/tokenserver/process_account_events.py
COPY --from=builder /app/tools/tokenserver/requirements.txt /app/tools/tokenserver/requirements.txt
COPY --from=builder /app/scripts/prepare-spanner.sh /app/scripts/prepare-spanner.sh
COPY --from=builder /app/syncstorage/src/db/spanner/schema.ddl /app/schema.ddl

RUN chmod +x /app/scripts/prepare-spanner.sh
RUN pip3 install -r /app/tools/integration_tests/requirements.txt
RUN pip3 install -r /app/tools/tokenserver/requirements.txt

USER app:app

ENTRYPOINT ["/app/bin/syncstorage", "--config=spanner_config.ini"]
