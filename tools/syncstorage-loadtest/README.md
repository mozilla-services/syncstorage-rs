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

## Docker

To run it inside docker:

```bash
docker run -e TEST_REPO=https://github.com/mozilla-services/syncstorage-loadtest -e TEST_NAME=test tarekziade/molotov:latest
```

Happy Breaking!
