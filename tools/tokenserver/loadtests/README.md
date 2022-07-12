# Tokenserver Load Tests

This directory contains everything needed to run the suite of load tests for Tokenserver.

## Building and Running
1. Install the load testing dependencies:
    ```sh
    pip3 install -r requirements.txt
    ```
2. Generate a private RSA key to be used by the load tests to sign tokens:
    ```sh
    openssl genrsa -out test.pem 2048
    ```
3. Derive the public key from the pem file generated in the previous step:
    ```sh
    openssl rsa -in test.pem -pubout > test.pub
    ```
4. Using the `authlib` Python library, derive the public JWK from the public key obtained in the previous step:
    ```python
    from authlib.jose import jwk

    public_key = open("test.pub", "rb").read()
    jwk.dumps(public_key, kty="RSA")
    ```
    This will give you a key of the form
    ```json
    {
        "n": ...,
        "e": ...,
        "kty": "RSA"
    }
    ```
5. Set the following environment variables/settings on Tokenserver:
    ```sh
    # Should be set to the "n" component of the JWK
    SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK_N
    # Should be set to the "e" component of the JWK (this value should almost always be "AQAB")
    SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK_E
    SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK_KTY=RSA
    SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK_USE=sig
    SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK_ALG=RS256

    # These two environment variables don't affect the load tests, but they need to be set:
    SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK_KID=""
    SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK_FXA_CREATED_AT=0
    ```
6. Configure Tokenserver to verify BrowserID assertions through FxA stage. This is done by setting the following environment variables:
    ```sh
    # The exact value of this environment variable is not important as long as it matches the `BROWSERID_AUDIENCE` environment variable set on the machine running the load tests, as described below
    SYNC_TOKENSERVER__FXA_BROWSERID_SERVER_URL=https://verifier.stage.mozaws.net/v2

    SYNC_TOKENSERVER__FXA_BROWSERID_AUDIENCE=https://token.stage.mozaws.net
    SYNC_TOKENSERVER__FXA_BROWSERID_ISSUER=mockmyid.s3-us-west-2.amazonaws.com
    ```
    Note that, because we have cached the JWK used to verify OAuth tokens, no verification requests will be made to FxA, so the value of `SYNC_TOKENSERVER__FXA_OAUTH_VERIFIER_URL` does not matter; however, Tokenserver expects it to be set, so setting it to something like `http://localhost` will suffice.
7. Set the following environment variables on the machine that will be running the load tests:
    * `OAUTH_PEM_FILE` should be set to the location of the private RSA key generated in a previous step
    * `BROWSERID_AUDIENCE` should be set to match the `SYNC_TOKENSERVER__FXA_BROWSERID_AUDIENCE` environment variable on Tokenserver
8. Tokenserver uses [locust](https://locust.io/) for load testing. To run the load tests, simply run the following command in this directory:
    ```sh
    locust
    ```
9. Navigate your browser to http://localhost:8090, where you'll find the locust GUI. Enter the following information:
    * Number of users: The peak number of Tokenserver users to be used during the load tests
    * Spawn rate: The rate at which new users are spawned
    * Host: The URL of the server to be load tested. Note that this URL must include the protocol (e.g. "http://")

10. Click the "Start swarming" button to begin the load tests.

## Populating the Database
This directory includes an optional `populate_db.py` script that can be used to add test users to the database en masse. The script can be run like so:
```sh
python3 populate_db.py <sqlurl> <nodes> <number of users>
```
where `sqluri` is the URL of the Tokenserver database, `nodes` is a comma-separated list of nodes **that are already present in the database** to which the users will be randomly assigned, and `number of users` is the number of users to be created. 
