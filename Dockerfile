ARG DATABASE_BACKEND=spanner
# Alternatively MYSQLCLIENT_PKG=libmysqlclient-dev for the Oracle/MySQL official client
ARG MYSQLCLIENT_PKG=libmariadb-dev-compat

# NOTE: Ensure builder's Rust version matches CI's in .circleci/config.yml
# RUST_VER
FROM docker.io/lukemathwalker/cargo-chef:0.1.71-rust-1.86-bullseye AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS cacher
ARG DATABASE_BACKEND
ARG MYSQLCLIENT_PKG

# cmake is required to build grpcio-sys for Spanner builds
RUN \
    if [ "$MYSQLCLIENT_PKG" = libmysqlclient-dev ] ; then \
        # Fetch and load the MySQL public key.
        wget -qO- https://repo.mysql.com/RPM-GPG-KEY-mysql-2023 > /etc/apt/trusted.gpg.d/mysql.asc && \
        echo "deb https://repo.mysql.com/apt/debian/ bullseye mysql-8.0" >> /etc/apt/sources.list ; \
    fi && \
    apt-get -q update && \
    apt-get -q install -y --no-install-recommends $MYSQLCLIENT_PKG cmake

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --no-default-features --features=syncstorage-db/$DATABASE_BACKEND --features=py_verifier --recipe-path recipe.json

FROM chef AS builder
ARG DATABASE_BACKEND
ARG MYSQLCLIENT_PKG

COPY . /app
COPY --from=cacher /app/target /app/target
COPY --from=cacher $CARGO_HOME /app/$CARGO_HOME

RUN \
    if [ "$MYSQLCLIENT_PKG" = libmysqlclient-dev ] ; then \
        # Fetch and load the MySQL public key.
        # mysql_pubkey.asc from:
        # https://dev.mysql.com/doc/refman/8.0/en/checking-gpg-signature.html
        # related:
        # https://dev.mysql.com/doc/mysql-apt-repo-quick-guide/en/#repo-qg-apt-repo-manual-setup
        wget -qO- https://repo.mysql.com/RPM-GPG-KEY-mysql-2023 > /etc/apt/trusted.gpg.d/mysql.asc && \
        echo "deb https://repo.mysql.com/apt/debian/ bullseye mysql-8.0" >> /etc/apt/sources.list ; \
    fi && \
    apt-get -q update && \
    apt-get -q install -y --no-install-recommends $MYSQLCLIENT_PKG cmake golang-go python3-dev python3-pip python3-setuptools python3-wheel && \
    pip3 install -r requirements.txt && \
    rm -rf /var/lib/apt/lists/*

ENV PATH=$PATH:/root/.cargo/bin

RUN \
    cargo --version && \
    rustc --version && \
    cargo install --path ./syncserver --no-default-features --features=syncstorage-db/$DATABASE_BACKEND --features=py_verifier --locked --root /app

FROM docker.io/library/debian:bullseye-slim
ARG MYSQLCLIENT_PKG

WORKDIR /app
COPY --from=builder /app/requirements.txt /app

RUN \
    apt-get -q update && apt-get -qy install wget
RUN \
    groupadd --gid 10001 app && \
    useradd --uid 10001 --gid 10001 --home /app --create-home app && \
    if [ "$MYSQLCLIENT_PKG" = libmysqlclient-dev ] ; then \
        # first, an apt-get update is required for gnupg, which is required for apt-key adv
        apt-get -q update && \
        # and ca-certificates needed for https://repo.mysql.com
        apt-get install -y gnupg ca-certificates wget && \
        # Fetch and load the MySQL public key
        echo "deb https://repo.mysql.com/apt/debian/ bullseye mysql-8.0" >> /etc/apt/sources.list && \
        wget -qO- https://repo.mysql.com/RPM-GPG-KEY-mysql-2023 > /etc/apt/trusted.gpg.d/mysql.asc ; \
    fi && \
    # update again now that we trust repo.mysql.com
    apt-get -q update && \
    apt-get -q install -y build-essential $MYSQLCLIENT_PKG libssl-dev libffi-dev libcurl4 python3-dev python3-pip python3-setuptools python3-wheel cargo curl jq && \
    # The python3-cryptography debian package installs version 2.6.1, but we
    # we want to use the version specified in requirements.txt. To do this,
    # we have to remove the python3-cryptography package here.
    apt-get -q remove -y python3-cryptography && \
    pip3 install -r /app/requirements.txt && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/bin /app/bin
COPY --from=builder /app/syncserver/version.json /app
COPY --from=builder /app/tools/spanner /app/tools/spanner
COPY --from=builder /app/tools/integration_tests /app/tools/integration_tests
COPY --from=builder /app/tools/tokenserver /app/tools/tokenserver
COPY --from=builder /app/scripts/prepare-spanner.sh /app/scripts/prepare-spanner.sh
COPY --from=builder /app/scripts/start_mock_fxa_server.sh /app/scripts/start_mock_fxa_server.sh
COPY --from=builder /app/syncstorage-spanner/src/schema.ddl /app/schema.ddl

RUN chmod +x /app/scripts/prepare-spanner.sh
RUN pip3 install -r /app/tools/integration_tests/requirements.txt
RUN pip3 install -r /app/tools/tokenserver/requirements.txt

USER app:app

ENTRYPOINT ["/app/bin/syncserver"]
