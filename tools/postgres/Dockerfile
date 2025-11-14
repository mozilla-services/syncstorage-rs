# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

FROM python:3.13-bookworm

ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    PIP_NO_CACHE_DIR=off \
    PIP_DISABLE_PIP_VERSION_CHECK=on \
    PIP_DEFAULT_TIMEOUT=100 \
    POETRY_VERSION=2.1.3 \
    POETRY_NO_INTERACTION=1 \
    POETRY_VIRTUALENVS_CREATE=false \
    POETRY_CACHE_DIR='/var/cache/pypoetry' \
    POETRY_HOME='/usr/local' \
    SQLALCHEMY_SILENCE_UBER_WARNING=1

RUN curl -sSL https://install.python-poetry.org | python3 -

WORKDIR /app/

COPY purge_ttl.py pyproject.toml /app/
RUN python3 --version

RUN poetry config virtualenvs.create false && \
    poetry install --without dev --no-interaction --no-ansi

USER nobody

ENTRYPOINT ["/usr/local/bin/python"]
CMD ["/app/purge_ttl.py"]
