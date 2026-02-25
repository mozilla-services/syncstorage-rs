# Tokenserver Load Tests

This directory contains everything needed to run the suite of load tests for Tokenserver.

## Prerequisite Dependency and Environment Setup:
To use the syncstorage-rs tokenserver load tests, you'll need a Python =>3.12 development environment with `Poetry` installed. This testing script can be run within a spun up GCP workflow where you can execute the locust commands from the UI. If you're running them ad-hoc, you'll need to follow these instructions. You can also directly call the script using `Poetry` as described in step 5.

The easiest solution recommended to use `pyenv` and the `pyenv-virtualenv` plugin for your virtual environments
as a way to isolate the dependencies from other directories.
1. Install `pyenv` using the [latest documentation](https://github.com/pyenv/pyenv#installation) for your platform.
2. Follow the instructions to install the `pyenv-virtualenv` plugin.
See the [pyenv-virtualenv](https://github.com/pyenv/pyenv-virtualenv) documentation.
3. Ensure you've added `pyenv` and `pyenv-virtualenv` to your PATH.

    Ex:
    ```shell
    export PATH="$HOME/.pyenv/bin:$PATH"
    eval "$(pyenv init -)"
    eval "$(pyenv virtualenv-init -)"
    ```
4. Install version, create virtualenv, activate and install dependencies from inside the `loadtests/` directory.
**Note** you can simply install dependencies, not create a virtual environment and invoke the script using `poetry run`.

    ```shell
    $ cd syncstorage-rs/tools/tokenserver/loadtests
    # pyenv version install
    $ pyenv install 3.10

    # creates named, associated virtualenv
    $ pyenv virtualenv 3.10 loadtests # or whatever project name you like.
    $ pyenv local loadtests # activates virtual env whenever you enter this directory. 

    # Install dependencies
    $ pip install poetry
    $ poetry install
    ```

5. In general, to run the script with the Poetry managed dependencies - once you're already in your virtual env - run the following (more details in #3):
Ex. `poetry run python locustfile.py`


## Building and Running

1. Install the load testing dependencies as described above:

   ```sh
   poetry install
   ```

2. Run the `generate-keys.sh` script to generate an RSA keypair and derive the public JWK.

Since this script calls `get_jwk.py` and it has a dependency on `autlib`, call the shell script using Poetry:

```sh
poetry run ./generate-keys.sh 
```

Otherwise, if in built virtual environment with installed Poetry dependencies:

```sh
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

3. Set the following environment variables/settings on Tokenserver:

   ```sh
   # Should be set to the "n" component of the JWK
   SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__N
   # Should be set to the "e" component of the JWK (this value should almost always be "AQAB")
   SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__E
   SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__KTY=RSA
   SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__USE=sig
   SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__ALG=RS256

   # These two environment variables don't affect the load tests, but they need to be set:
   SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__KID=""
   SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__FXA_CREATED_AT=0
   ```

   Note that, because these settings cache the JWK used to verify OAuth tokens, no verification requests will be made to FxA, so the value of `SYNC_TOKENSERVER__FXA_OAUTH_VERIFIER_URL` does not matter; however, Tokenserver expects it to be set, so setting it to something like `http://localhost` will suffice.

4. Set the following environment variables on the machine that will be running the load tests:

   - `OAUTH_PEM_FILE` should be set to the location of the private RSA key generated in a previous step

5. Tokenserver uses [locust](https://locust.io/) for load testing. To run the load tests, simply run the following command in this directory:

   ```sh
   locust
   ```

6. Navigate your browser to <http://localhost:8090>, where you'll find the locust GUI. Enter the following information:

   - Number of users: The peak number of Tokenserver users to be used during the load tests
   - Spawn rate: The rate at which new users are spawned
   - Host: The URL of the server to be load tested. Note that this URL must include the protocol (e.g. "http://")

7. Click the "Start swarming" button to begin the load tests.

## Populating the Database

This directory includes an optional `populate_db.py` script that can be used to add test users to the database en masse. The script can be run like so:

```sh
poetry run python populate_db.py <sqlurl> <nodes> <number of users>
```

Or if in built virtualenv:
```sh
python3 populate_db.py <sqlurl> <nodes> <number of users>
```

where `sqluri` is the URL of the Tokenserver database, `nodes` is a comma-separated list of nodes **that are already present in the database** to which the users will be randomly assigned, and `number of users` is the number of users to be created.
