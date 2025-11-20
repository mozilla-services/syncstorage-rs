# PostgreSQL Utility Scripts

Currently there is one script: `purge_ttl.py`.

## purge_ttl.py

Purge expired items (BSOs and batches) from a Sync storage PostgreSQL instance.
The script is intended to be invoked regularly, e.g. as a cronjob.

### Prerequisites

- Python 3
- [Poetry](https://python-poetry.org/docs/#installation)

### Run Script Locally

```bash
cd tools/postgres
poetry install
SYNC_SYNCSTORAGE__DATABASE_URL="postgresql://user:pass@localhost/syncstorage poetry run python purge_ttl.py"
```

Pass `-h` to see the list of options.

### Run Tests Locally

```bash
cd tools/postgres
poetry install
SYNC_SYNCSTORAGE__DATABASE_URL="postgresql://user:pass@localhost/syncstorage poetry run pytest test_purge_ttl.py -v"
```

### Run with Docker

A Docker image with the PostgreSQL build is not published anywhere at the
moment.  Once that's available, this readme will be updated to include that
step.

```bash
docker run --rm \
  --entrypoint python3 \
  -e SYNC_SYNCSTORAGE__DATABASE_URL="postgresql://user:pass@host:5432/syncstorage" \
  syncstorage-postgres-image:tag \
  /app/tools/postgres/purge_ttl.py
```

