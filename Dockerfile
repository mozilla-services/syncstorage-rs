FROM rust:1.64-buster as builder
WORKDIR /app
ADD . /app
ENV PATH=$PATH:/root/.cargo/bin
# temp removed --no-install-recommends due to CI docker build issue
RUN \
    echo "deb https://repo.mysql.com/apt/debian/ buster mysql-8.0" >> /etc/apt/sources.list && \
    # mysql_pubkey.asc from:
    # https://dev.mysql.com/doc/refman/8.0/en/checking-gpg-signature.html
    # related:
    # https://dev.mysql.com/doc/mysql-apt-repo-quick-guide/en/#repo-qg-apt-repo-manual-setup
    apt-key adv --import mysql_pubkey.asc && \
    apt-get -q update && \
    apt-get -q install -y --no-install-recommends libmysqlclient-dev cmake golang-go python3-dev python3-pip python3-setuptools python3-wheel && \
    pip3 install -r requirements.txt && \
    rm -rf /var/lib/apt/lists/*

RUN \
    cargo --version && \
    rustc --version && \
    cargo install --path ./syncserver --locked --root /app && \
    cargo install --path ./syncserver --locked --root /app --bin purge_ttl

FROM debian:buster-slim
WORKDIR /app
COPY --from=builder /app/requirements.txt /app
COPY --from=builder /app/mysql_pubkey.asc /app
# Due to a build error that occurs with the Python cryptography package, we
# have to set this env var to prevent the cryptography package from building
# with Rust. See this link for more information:
# https://pythonshowcase.com/question/problem-installing-cryptography-on-raspberry-pi
ENV CRYPTOGRAPHY_DONT_BUILD_RUST=1
RUN \
    groupadd --gid 10001 app && \
    useradd --uid 10001 --gid 10001 --home /app --create-home app && \
    # first, an apt-get update is required for gnupg, which is required for apt-key adv
    apt-get -q update && \
    # and ca-certificates needed for https://repo.mysql.com
    apt-get install -y gnupg ca-certificates && \
    echo "deb https://repo.mysql.com/apt/debian/ buster mysql-8.0" >> /etc/apt/sources.list && \
    apt-key adv --import mysql_pubkey.asc && \
    # update again now that we trust repo.mysql.com
    apt-get -q update && \
    apt-get -q install -y build-essential libmysqlclient-dev libssl-dev libffi-dev libcurl4 python3-dev python3-pip python3-setuptools python3-wheel cargo curl jq && \
    # The python3-cryptography debian package installs version 2.6.1, but we
    # we want to use the version specified in requirements.txt. To do this,
    # we have to remove the python3-cryptography package here.
    apt-get -q remove -y python3-cryptography && \
    pip3 install -r /app/requirements.txt && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/bin /app/bin
COPY --from=builder /app/syncserver/version.json /app
COPY --from=builder /app/spanner_config.ini /app
COPY --from=builder /app/tools/spanner /app/tools/spanner
COPY --from=builder /app/tools/integration_tests /app/tools/integration_tests
COPY --from=builder /app/tools/tokenserver/process_account_events.py /app/tools/tokenserver/process_account_events.py
COPY --from=builder /app/tools/tokenserver/requirements.txt /app/tools/tokenserver/requirements.txt
COPY --from=builder /app/scripts/prepare-spanner.sh /app/scripts/prepare-spanner.sh
COPY --from=builder /app/syncserver/src/db/spanner/schema.ddl /app/schema.ddl

RUN chmod +x /app/scripts/prepare-spanner.sh
RUN pip3 install -r /app/tools/integration_tests/requirements.txt
RUN pip3 install -r /app/tools/tokenserver/requirements.txt

USER app:app

ENTRYPOINT ["/app/bin/syncserver", "--config=spanner_config.ini"]
