# Tokenserver Load Tests

This directory contains everything needed to run the suite of load tests for Tokenserver.

## Building and Running
1. Install the load testing dependencies:
```sh
pip3 install -r requirements.txt
```
1. Set up a mock FxA service, with which Tokenserver will verify OAuth tokens. This directory includes a `mock-oauth-cfn.yaml` file, which contains the AWS CloudFormation template needed to deploy the mock service. A service with this template has been preconfigured at [https://mock-oauth-stage.dev.lcip.org](), but you can deploy your own with the following command:
```sh
aws cloudformation deploy \
    --template-file=mock-oauth-cfn.yml \
    --stack-name my-mock-oauth-stack \
    --capabilities CAPABILITY_IAM \
    --parameter-overrides \
        DomainName=my-mock-oauth.dev.lcip.org
```
1. Configure Tokenserver to verify tokens through the mock FxA service. This is done by setting the `tokenserver.fxa_oauth_server_url` setting or `SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL` environment variable to the URL of the desired mock service.
1. Tokenserver uses [locust](https://locust.io/) for load testing. To run the load tests, simply run the following command in this directory:
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
