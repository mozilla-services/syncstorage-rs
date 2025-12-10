ARG SYNCSTORAGE_DATABASE_BACKEND=spanner
ARG TOKENSERVER_DATABASE_BACKEND=mysql
# Alternatively MYSQLCLIENT_PKG=libmysqlclient-dev for the Oracle/MySQL official client
ARG MYSQLCLIENT_PKG=libmariadb-dev-compat

# NOTE: Ensure builder's Rust version matches CI's in .circleci/config.yml
# RUST_VER
FROM docker.io/lukemathwalker/cargo-chef:0.1.72-rust-1.89-bookworm AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG SYNCSTORAGE_DATABASE_BACKEND
ARG TOKENSERVER_DATABASE_BACKEND
ARG MYSQLCLIENT_PKG

RUN apt-get -q update && \
    MYSQL_PKG="" && \
    POSTGRES_DEV_PKG="" && \
    if [ "$SYNCSTORAGE_DATABASE_BACKEND" = "mysql" ] || [ "$TOKENSERVER_DATABASE_BACKEND" = "mysql" ]; then \
        MYSQL_PKG="$MYSQLCLIENT_PKG"; \
        if [ "$MYSQLCLIENT_PKG" = libmysqlclient-dev ] ; then \
            # First install gnupg and setup MySQL repo
            # Key ID A8D3785C from https://dev.mysql.com/doc/refman/8.0/en/checking-gpg-signature.html
            apt-get -q install -y --no-install-recommends gnupg ca-certificates && \
            echo "deb https://repo.mysql.com/apt/debian/ bookworm mysql-8.0" >> /etc/apt/sources.list && \
            # Fetch and install the MySQL public key
            gpg --batch --keyserver hkp://keyserver.ubuntu.com --recv-keys A8D3785C && \
            gpg --batch --armor --export A8D3785C | tee /etc/apt/trusted.gpg.d/mysql.asc && \
            apt-get -q update ; \
        fi; \
    fi && \
    if [ "$TOKENSERVER_DATABASE_BACKEND" = "postgres" ]; then \
        POSTGRES_DEV_PKG="libpq-dev"; \
    fi && \
    apt-get -q install -y --no-install-recommends $MYSQL_PKG $POSTGRES_DEV_PKG cmake python3-dev python3-pip python3-setuptools python3-wheel python3-venv pkg-config && \
    rm -rf /var/lib/apt/lists/*

COPY --from=planner /app/recipe.json recipe.json

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    set -x && \
    TOKENSERVER_FEATURES="" && \
    if [ "$TOKENSERVER_DATABASE_BACKEND" = "postgres" ]; then \
        TOKENSERVER_FEATURES="--features=tokenserver-db/postgres"; \
    fi && \
    cargo chef cook --release --no-default-features --features=syncstorage-db/$SYNCSTORAGE_DATABASE_BACKEND $TOKENSERVER_FEATURES --features=py_verifier --recipe-path recipe.json

ENV POETRY_HOME="/opt/poetry" \
    POETRY_VIRTUALENVS_IN_PROJECT=1 \
    POETRY_NO_INTERACTION=1
ENV PATH="$POETRY_HOME/bin:$PATH"

RUN curl -sSL https://install.python-poetry.org | python3 - && \
    ln -s $POETRY_HOME/bin/poetry /usr/local/bin/poetry && \
    poetry --version && \
    poetry config virtualenvs.create false && \
    poetry self add poetry-plugin-export

COPY . /app

# Generating a requirements.txt from Poetry dependencies.
# [tool.poetry.dependencies]
RUN poetry export --no-interaction --without dev --output requirements.txt --without-hashes && \
    cd /app/tools/integration_tests && \
    poetry export --no-interaction --without dev --output requirements.txt --without-hashes && \
    cd /app/tools/tokenserver && \
    poetry export --no-interaction --without dev --output requirements.txt --without-hashes && \
    cd /app/tools/postgres && \
    if [ "$SYNCSTORAGE_DATABASE_BACKEND" = "postgres" ]; then \
        poetry export --no-interaction --without dev --output requirements.txt --without-hashes; \
    else \
        # Because we can't conditionally COPY files in the next stage, generate
        # this empty requirements.txt file so that we can always COPY it
        touch requirements.txt; \
    fi && \
    cd /app

# Build wheels for all Python dependencies so 
RUN mkdir -p /app/wheels && \
    pip3 wheel --no-cache-dir -r /app/requirements.txt -w /app/wheels && \
    pip3 wheel --no-cache-dir -r /app/tools/integration_tests/requirements.txt -w /app/wheels && \
    pip3 wheel --no-cache-dir -r /app/tools/tokenserver/requirements.txt -w /app/wheels && \
    if [ "$SYNCSTORAGE_DATABASE_BACKEND" = "postgres" ] && [ -f /app/tools/postgres/requirements.txt ]; then \
        pip3 wheel --no-cache-dir -r /app/tools/postgres/requirements.txt -w /app/wheels; \
    fi

ENV PATH=$PATH:/root/.cargo/bin

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    set -x && \
    TOKENSERVER_FEATURES="" && \
    if [ "$TOKENSERVER_DATABASE_BACKEND" = "postgres" ]; then \
        TOKENSERVER_FEATURES="--features=tokenserver-db/postgres"; \
    fi && \
    cargo --version && \
    rustc --version && \
    cargo install --path ./syncserver --no-default-features --features=syncstorage-db/$SYNCSTORAGE_DATABASE_BACKEND $TOKENSERVER_FEATURES --features=py_verifier --locked --root /app

FROM docker.io/library/debian:bookworm-slim
ARG SYNCSTORAGE_DATABASE_BACKEND
ARG TOKENSERVER_DATABASE_BACKEND
ARG MYSQLCLIENT_PKG

RUN apt-get -q update && \
    MYSQL_PKG="" && \
    POSTGRES_PKG="" && \
    # Always install MySQL libs because Python integration tests depend on mysqlclient
    MYSQL_PKG="$MYSQLCLIENT_PKG" && \
    if [ "$MYSQLCLIENT_PKG" = libmysqlclient-dev ] ; then \
        # First install gnupg and setup MySQL repo
        apt-get install -y --no-install-recommends gnupg ca-certificates wget && \
        echo "deb https://repo.mysql.com/apt/debian/ bookworm mysql-8.0" >> /etc/apt/sources.list && \
        # Fetch and install the MySQL public key
        gpg --batch --keyserver hkp://keyserver.ubuntu.com --recv-keys A8D3785C && \
        gpg --batch --armor --export A8D3785C | tee /etc/apt/trusted.gpg.d/mysql.asc && \
        apt-get -q update ; \
    fi && \
    POSTGRES_PKG="libpq5" && \
    apt-get -q install -y --no-install-recommends $MYSQL_PKG $POSTGRES_PKG libssl3 libffi8 libcurl4 libpython3.11 python3 python3-pip python3-venv curl jq && \
    # The python3-cryptography debian package installs version 2.6.1, but we
    # we want to use the version specified in requirements.txt. To do this,
    # we have to remove the python3-cryptography package here.
    apt-get -q remove -y python3-cryptography 2>/dev/null || true && \
    apt-get -q autoremove -y && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/requirements.txt /app/
COPY --from=builder /app/tools/integration_tests/requirements.txt /app/tools/integration_tests/
COPY --from=builder /app/tools/tokenserver/requirements.txt /app/tools/tokenserver/
# See comment above where this requirements file is generated
COPY --from=builder /app/tools/postgres/requirements.txt /app/tools/postgres/
COPY --from=builder /app/wheels /tmp/wheels

RUN groupadd --gid 10001 app && \
    useradd --uid 10001 --gid 10001 --home /app --create-home app

RUN pip3 install --break-system-packages --no-cache-dir --no-index --find-links=/tmp/wheels -r /app/requirements.txt && \
    pip3 install --break-system-packages --no-cache-dir --no-index --find-links=/tmp/wheels -r /app/tools/integration_tests/requirements.txt && \
    pip3 install --break-system-packages --no-cache-dir --no-index --find-links=/tmp/wheels -r /app/tools/tokenserver/requirements.txt && \
    if [ "$SYNCSTORAGE_DATABASE_BACKEND" = "postgres" ] && [ -f /app/tools/postgres/requirements.txt ]; then \
        pip3 install --break-system-packages --no-cache-dir --no-index --find-links=/tmp/wheels -r /app/tools/postgres/requirements.txt; \
    fi && \
    rm -rf /tmp/wheels /root/.cache/pip

COPY --from=builder /app/bin /app/bin
COPY --from=builder /app/syncserver/version.json /app
COPY --from=builder /app/tools/integration_tests /app/tools/integration_tests
COPY --from=builder --chmod=0755 /app/scripts/prepare-spanner.sh /app/scripts/prepare-spanner.sh
COPY --from=builder --chmod=0755 /app/scripts/start_mock_fxa_server.sh /app/scripts/start_mock_fxa_server.sh
COPY --from=builder /app/syncstorage-spanner/src/schema.ddl /app/schema.ddl

USER app:app

ENTRYPOINT ["/app/bin/syncserver"]
