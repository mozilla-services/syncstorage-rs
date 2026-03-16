# `make_hawk_token.py`

## Summary

`make_hawk_token.py` is a utility script for generating a [Hawk](https://github.com/mozilla/hawk) authentication token and header for use when testing the syncstorage-rs API. It derives a token and signing key from a shared secret and produces either a raw token tuple or a ready-to-use `Authorization: Hawk ...` header.

The script lives in [`tools/hawk/`](https://github.com/mozilla-services/syncstorage-rs/tree/master/tools/hawk).

---

## Setup

### Prerequisites

- Python ≥ 3.12
- [Poetry](https://python-poetry.org/docs/#installation) installed

### Install dependencies

```shell
cd tools/hawk
poetry install
```

---

## Usage

```shell
poetry run python make_hawk_token.py [options]
```

By default (no arguments), a token is generated for `http://localhost:8000/1.5/1/storage/col2/` using a built-in test secret.

### Options

| Option | Description | Default |
|---|---|---|
| `--uri` | Request URI path | `/1.5/1/storage/col2/` |
| `--secret` | Shared `SYNC_MASTER_SECRET` value | *(built-in test value)* |
| `--method` | HTTP method for the request | `GET` |
| `--as_header` | Print a ready-to-use `Authorization: Hawk ...` header | *(off)* |

Use `-h` for the full option list.

---

## Examples

### Generate a token for a GET request

```shell
poetry run python make_hawk_token.py \
  --uri /1.5/1/storage/meta/global \
  --secret $SYNC_MASTER_SECRET
```

### Generate a ready-to-use Authorization header

```shell
poetry run python make_hawk_token.py \
  --uri /1.5/1/storage/meta/global \
  --secret $SYNC_MASTER_SECRET \
  --as_header
```

Output:
```
Authorization: Hawk id="...", ts="...", nonce="...", mac="..."
```

### Non-GET methods

For methods other than GET (e.g. PUT, POST, DELETE), pass `--method`:

```shell
poetry run python make_hawk_token.py \
  --method PUT \
  --uri /1.5/1/storage/meta/global \
  --secret $SYNC_MASTER_SECRET \
  --as_header
```

See [`tools/examples/put.bash`](https://github.com/mozilla-services/syncstorage-rs/blob/master/tools/examples/put.bash) for a full curl example using this output.

---

## Notes

- The `--secret` value must match the `SYNC_MASTER_SECRET` configured in the running syncstorage-rs instance.
- The generated token is scoped to the exact URI and method — a token generated for `GET /foo` is not valid for `PUT /foo`.
