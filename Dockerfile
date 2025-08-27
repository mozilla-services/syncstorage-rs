ARG PYTHON_VERSION=3.13.0
ARG DATABASE_BACKEND=spanner
# Alternatively MYSQLCLIENT_PKG=libmysqlclient-dev for the Oracle/MySQL official client
ARG MYSQLCLIENT_PKG=libmariadb-dev-compat

# NOTE: Ensure builder's Rust version matches CI's in .circleci/config.yml
# RUST_VER
FROM docker.io/lukemathwalker/cargo-chef:0.1.72-rust-1.89-bullseye AS chef
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

ENV POETRY_HOME="/opt/poetry" \
    POETRY_VIRTUALENVS_IN_PROJECT=1 \
    POETRY_NO_INTERACTION=1

ENV PATH="$POETRY_HOME/bin:$PATH"

COPY . /app
COPY --from=cacher /app/target /app/target
COPY --from=cacher $CARGO_HOME /app/$CARGO_HOME

# Install Python 3.13 manually
ENV PYTHON_VERSION=3.13.0

RUN apt-get update && apt-get install -y \
    wget build-essential libssl-dev zlib1g-dev libbz2-dev \
    libreadline-dev libsqlite3-dev curl libncursesw5-dev \
    xz-utils tk-dev libxml2-dev libxmlsec1-dev libffi-dev liblzma-dev

RUN curl -O https://www.python.org/ftp/python/$PYTHON_VERSION/Python-$PYTHON_VERSION.tgz && \
    tar xzf Python-$PYTHON_VERSION.tgz && \
    cd Python-$PYTHON_VERSION && \
    ./configure --enable-optimizations && \
    make -j"$(nproc)" && \
    make altinstall && \
    ln -sf /usr/local/bin/python3.13 /usr/local/bin/python3 && \
    ln -sf /usr/local/bin/pip3.13 /usr/local/bin/pip3 && \
    cd .. && rm -rf Python-$PYTHON_VERSION*

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
    apt-get -q install -y --no-install-recommends $MYSQLCLIENT_PKG cmake golang-go pkg-config && \
    rm -rf /var/lib/apt/lists/*

RUN curl -sSL https://install.python-poetry.org | python3 - && \
    ln -s $POETRY_HOME/bin/poetry /usr/local/bin/poetry && \
    poetry --version && \
    poetry config virtualenvs.create false && \
    poetry self add poetry-plugin-export

# Generating a requirements.txt from Poetry dependencies.
# [tool.poetry.dependencies]
RUN poetry export --no-interaction --without dev --output requirements.txt --without-hashes && \
    pip3 install -r requirements.txt


ENV PATH=$PATH:/root/.cargo/bin

RUN \
    cargo --version && \
    rustc --version && \
    cargo install --path ./syncserver --no-default-features --features=syncstorage-db/$DATABASE_BACKEND --features=py_verifier --locked --root /app

FROM python:${PYTHON_VERSION}-slim
ARG MYSQLCLIENT_PKG

ENV POETRY_HOME="/opt/poetry" \
    POETRY_VIRTUALENVS_IN_PROJECT=1 \
    POETRY_NO_INTERACTION=1

ENV PATH="$POETRY_HOME/bin:$PATH"

WORKDIR /app
COPY --from=builder /app/requirements.txt /app
COPY --from=builder /app/pyproject.toml /app/poetry.lock /app/

RUN apt-get -q update && apt-get -qy install wget
RUN groupadd --gid 10001 app && \
    useradd --uid 10001 --gid 10001 --home /app --create-home app
RUN if [ "$MYSQLCLIENT_PKG" = libmysqlclient-dev ] ; then \
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
    apt-get -q install -y build-essential $MYSQLCLIENT_PKG libssl-dev libffi-dev libcurl4 cargo curl jq pkg-config && \
    # The python3-cryptography debian package installs version 2.6.1, but we
    # we want to use the version specified in requirements.txt. To do this,
    # we have to remove the python3-cryptography package here.
    apt-get -q remove -y python3-cryptography && \
    rm -rf /var/lib/apt/lists/*

RUN curl -sSL https://install.python-poetry.org | python3 - && \
    ln -s $POETRY_HOME/bin/poetry /usr/local/bin/poetry && \
    poetry --version && \
    poetry config virtualenvs.create false && \
    poetry self add poetry-plugin-export
# Generating a requirements.txt from Poetry dependencies.
# [tool.poetry.dependencies]
RUN poetry export --no-interaction --without dev --output requirements.txt --without-hashes && \
    pip3 install -r requirements.txt

COPY --from=builder /app/bin /app/bin
COPY --from=builder /app/syncserver/version.json /app
COPY --from=builder /app/tools/spanner /app/tools/spanner
COPY --from=builder /app/tools/integration_tests /app/tools/integration_tests
COPY --from=builder /app/tools/tokenserver /app/tools/tokenserver
COPY --from=builder /app/scripts/prepare-spanner.sh /app/scripts/prepare-spanner.sh
COPY --from=builder /app/scripts/start_mock_fxa_server.sh /app/scripts/start_mock_fxa_server.sh
COPY --from=builder /app/syncstorage-spanner/src/schema.ddl /app/schema.ddl

RUN chmod +x /app/scripts/prepare-spanner.sh

WORKDIR /app/tools/integration_tests/
RUN poetry export --no-interaction --without dev --output requirements.txt --without-hashes
WORKDIR /app/tools/tokenserver/
RUN poetry export --no-interaction --without dev --output requirements.txt --without-hashes
WORKDIR /app
RUN pip3 install -r /app/tools/integration_tests/requirements.txt
RUN pip3 install -r /app/tools/tokenserver/requirements.txt

USER app:app

ENTRYPOINT ["/app/bin/syncserver"]
