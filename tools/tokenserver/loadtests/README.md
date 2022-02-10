# Tokenserver Load Tests

This directory contains everything needed to run the suite of load tests for Tokenserver.

## Building and Running
1. Install the load testing dependencies:
```sh
pip3 install -r requirements.txt
```
2. Set up a mock FxA service, with which Tokenserver will verify OAuth tokens and BrowserID assertions. The subdirectory [mock-fxa-server/](./mock-fxa-server) includes code deployable as a GCP Cloud Function that acts as a mock FxA server, "verifying" OAuth tokens and BrowserID assertions. You can deploy your own Cloud Function by running the following command in this directory:
```sh
gcloud functions deploy mock_fxa_server --runtime=python39 --trigger-http --source=mock-fxa-server
```
You can stand up a local copy of the Cloud Function by running the following in this directory:
```sh
functions-framework --target mock_fxa_server --debug
```
Note that you'll need to install `functions-framework` via `pip3 install functions-framework`.

3. Configure Tokenserver to verify tokens through the mock FxA service. This is done by setting the `tokenserver.fxa_oauth_server_url` setting or `SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL` environment variable to the URL of the desired mock service.
4. Tokenserver uses [locust](https://locust.io/) for load testing. To run the load tests, simply run the following command in this directory:
```sh
locust
```
Next, navigate your browser to http://localhost:8090, where you'll find the locust GUI. Enter the following information:
* Number of users: The peak number of simultaneous connections to Tokenserver
* Spawn rate: The rate at which new connections are created
* Host: The URL of the server to be load tested. Note that this URL must include the protocol (e.g. "http://").

Click the "Start swarming" button to begin the load tests.

## Populating the Database
This directory includes an optional `populate_db.py` script that can be used to add test users to the database en masse. The script can be run like so:
```sh
python3 populate_db.py <sqlurl> <nodes> <number of users>
```
where `sqluri` is the URL of the Tokenserver database, `nodes` is a comma-separated list of nodes **that are already present in the database** to which the users will be randomly assigned, and `number of users` is the number of users to be created. 
