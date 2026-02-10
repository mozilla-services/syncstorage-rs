# SyncStorage-LoadTest

FROM python:3.10-slim

RUN mkdir -p /app
ADD . /app
WORKDIR /app

# Building:
# you can build a local docker image using
# `docker build . --tag syncstorage-loadtest:local`

# system setup
RUN \
    BUILD_DEPS="git build-essential" && \
    # wget not required but nice to have
    RUN_DEPS="wget libssl-dev" && \
    apt-get update && \
    apt-get install -yq --no-install-recommends ${BUILD_DEPS} ${RUN_DEPS} && \
    pip install --no-cache-dir poetry && \
    apt-get purge -yq --auto-remove ${BUILD_DEPS} && \
    apt-get autoremove -yqq && \
    apt-get clean -y

# app install
RUN poetry config virtualenvs.create false && \
    poetry install --no-dev --no-interaction --no-ansi

# Using:
# Start an interactive terminal using
# `docker run --net=host -it syncstorage-loadtest:local`
# This will start a bash shell as root.
# You can fire off a load test by calling:
# `SERVER_URL=http://${HOST}:${PORT}#${SECRET} molotov -v`

ENTRYPOINT ["/bin/bash"]
