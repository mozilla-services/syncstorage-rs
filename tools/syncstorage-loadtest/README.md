# Storage load test

**Note:** This requires Python version 3.10+

The tests can execute in three modes:

1. **Direct access**: use a known master secret
2. **Firefox accounts OAuth**: create test accounts and use OAuth JWTs
3. **Self-signed JWT**: generate and sign JWTs locally with a given private key

## Installation

### Environment Setup

To run the syncstorage load tests, you'll need a Python >=3.10 development environment with `Poetry` installed. You can also directly use `poetry run` to execute commands as described in the usage examples below.

The easiest solution is to use `pyenv` and the `pyenv-virtualenv` plugin for your virtual environments as a way to isolate the dependencies from other projects.

1. Install `pyenv` using the [latest documentation](https://github.com/pyenv/pyenv#installation) for your platform.

2. Follow the instructions to install the `pyenv-virtualenv` plugin.
   See the [pyenv-virtualenv](https://github.com/pyenv/pyenv-virtualenv) documentation.

3. Ensure you've added `pyenv` and `pyenv-virtualenv` to your PATH.

   Example:
   ```shell
   export PATH="$HOME/.pyenv/bin:$PATH"
   eval "$(pyenv init -)"
   eval "$(pyenv virtualenv-init -)"
   ```

4. Install Python version, create virtualenv, activate and install dependencies from inside the project directory.

   **Note:** You can skip creating a virtual environment and invoke commands directly using `poetry run`.

   ```shell
   $ cd syncstorage-loadtest

   # Install Python 3.10+
   $ pyenv install 3.10

   # Create named, associated virtualenv
   $ pyenv virtualenv 3.10 syncstorage-loadtest  # or whatever name you prefer
   $ pyenv local syncstorage-loadtest  # activates virtual env whenever you enter this directory

   # Install Poetry and dependencies
   $ pip install poetry
   $ poetry install
   ```

5. Once you're in your virtual environment, run the load tests using:
   ```bash
   poetry run molotov [options] loadtest.py
   ```

### Quick Install

If you already have Poetry installed:

```bash
poetry install
```
### Generate Key
 Run the `generate-keys.sh` script to generate an RSA keypair and derive the public JWK.

Since this script calls `get_jwk.py` and it has a dependency on `autlib`, call the shell script using Poetry:

```bash
poetry run ./generate-keys.sh
```

Otherwise, if in built virtual environment with installed Poetry dependencies:

```bash
./generate-keys.sh
```

This script will output two files:

- `load_test.pem`: The private key to be used by the load tests to create OAuth tokens
- `jwk.json`: The public JWK associated with the private key. This is a key of the form:

```json
{
   "n": ...,
   "e": ...,
   "kty": "RSA"
}
```
## Mode 1: Direct Access

With a known syncstorage master secret:

```bash
SERVER_URL="http://localhost:8000#secretValue" poetry run molotov --max-runs 5 -cxv loadtest.py
```

## Mode 2: Firefox Accounts OAuth

With FxA stage accounts:

```bash
SERVER_URL="https://token.stage.mozaws.net" \
  FXA_API_HOST="https://api-accounts.stage.mozaws.net" \
  FXA_OAUTH_HOST="https://oauth.stage.mozaws.net" \
  poetry run molotov --workers 3 --duration 60 -v loadtest.py
```

## Mode 3: Self-Signed JWTs

Assuming an RSA keypair was generated, the private key pem was saved and accessible, and the token server was started with the public key JWK configured. The presence of `OAUTH_PRIVATE_KEY_FILE` triggers this mode when using OAuth.

Run with self-signed JWTs:

```bash
SERVER_URL="http://localhost:8000" \
  OAUTH_PRIVATE_KEY_FILE="/path/to/load_test.pem" \
  poetry run molotov --workers 100 --duration 300 -v loadtest.py
```

## Expanded payloads (GCS offload)

The batch write path can generate larger-than-default payloads so you can load
test collections whose payloads are offloaded to Google Cloud Storage. These
knobs are opt-in via environment variables; unset, the load test behaves
exactly as before.

| Variable | Default | Effect |
|---|---|---|
| `LARGE_PAYLOAD_PROB` | `0.0` | Fraction (`0.0`–`1.0`) of BSOs given an expanded payload. `1.0` makes **every** payload large; `0.0` disables the feature. |
| `LARGE_PAYLOAD_SIZE` | *(unset)* | Explicit target size in bytes for large payloads. When unset, large payloads target the server's `max_record_payload_bytes`. |
| `OFFLOAD_COLLECTIONS` | *(unset)* | Comma-separated collection names to target for batch writes. Must match the server's `gcs_payload_offload_collections`. When unset, the default collections are used. |

`LARGE_PAYLOAD_PROB`/`LARGE_PAYLOAD_SIZE` control payload **size**; whether a
write is offloaded to GCS is a separate, per-collection server setting. To drive
a fully offloaded collection at maximum size, set `OFFLOAD_COLLECTIONS` to that
collection and `LARGE_PAYLOAD_PROB=1.0`.

Individual payloads are capped at the target collection's
`max_record_payload_bytes`, and each batch is kept under its `max_post_bytes`,
so a large enough payload results in fewer records per request.

Example — every write is a 5 MiB payload sent to the `tabs` collection:

```bash
SERVER_URL="http://localhost:8000" \
  OAUTH_PRIVATE_KEY_FILE="/path/to/load_test.pem" \
  LARGE_PAYLOAD_PROB=1.0 \
  LARGE_PAYLOAD_SIZE=5242880 \
  OFFLOAD_COLLECTIONS=tabs \
  poetry run molotov --workers 100 --duration 300 -v loadtest.py
```

### Server-side prerequisites

To actually exercise expanded/offloaded payloads end to end, the target server
must be configured for it:

- `gcs_payload_bucket` set and the target collection listed in
  `gcs_payload_offload_collections` (syncstorage settings).
- Raised limits to allow payloads beyond the 2.5 MiB default:
  `max_record_payload_bytes`, `max_post_bytes`, `max_request_bytes`, and
  `max_total_bytes`. These are read from `/info/configuration`. A collection
  may also raise its own limits via the `collections` section of that
  response; the load test resolves limits per target collection, preferring a
  collection's override and falling back to the global value.
- Any front-end proxy (e.g. nginx) request body-size limit raised to match.

Note: many workers each holding multi-MiB payloads is a real memory footprint
on the load-generating host — size the harness accordingly.

## Docker

To run it inside docker:

```bash
docker run -e TEST_REPO=https://github.com/mozilla-services/syncstorage-loadtest -e TEST_NAME=test tarekziade/molotov:latest
```

Happy Breaking!
