##
# Collection of helper scripts used for local dev.
##

SYNC_SYNCSTORAGE__DATABASE_URL = 'mysql://sample_user:sample_password@localhost/syncstorage_rs'
SYNC_TOKENSERVER__DATABASE_URL = 'mysql://sample_user:sample_password@localhost/tokenserver_rs'

# This key can live anywhere on your machine. Adjust path as needed.
PATH_TO_SYNC_SPANNER_KEYS = `pwd`/service-account.json

# TODO: replace with rust grpc alternative when ready
# Assumes you've cloned the server-syncstorage repo locally into a peer dir.
# https://github.com/mozilla-services/server-syncstorage
PATH_TO_GRPC_CERT = ../server-syncstorage/local/lib/python2.7/site-packages/grpc/_cython/_credentials/roots.pem

clippy:
	# Matches what's run in circleci
	cargo clippy --all --all-targets --all-features -- -D warnings

clean:
	cargo clean
	rm -r venv

docker_start_mysql:
	docker-compose -f docker-compose.mysql.yaml up -d

docker_start_mysql_rebuild:
	docker-compose -f docker-compose.mysql.yaml up --build -d

docker_stop_mysql:
	docker-compose -f docker-compose.mysql.yaml down

docker_start_spanner:
	docker-compose -f docker-compose.spanner.yaml up -d

docker_start_spanner_rebuild:
	docker-compose -f docker-compose.spanner.yaml up --build -d

docker_stop_spanner:
	docker-compose -f docker-compose.spanner.yaml down

python:
	python3 -m venv venv
	venv/bin/python -m pip install -r requirements.txt

run: python
	PATH="./venv/bin:$(PATH)" RUST_LOG=debug RUST_BACKTRACE=full cargo run -- --config config/local.toml

run_spanner:
	GOOGLE_APPLICATION_CREDENTIALS=$(PATH_TO_SYNC_SPANNER_KEYS) GRPC_DEFAULT_SSL_ROOTS_FILE_PATH=$(PATH_TO_GRPC_CERT) make run

test:
	SYNC_SYNCSTORAGE__DATABASE_URL=$(SYNC_SYNCSTORAGE__DATABASE_URL) SYNC_TOKENSERVER__DATABASE_URL=$(SYNC_TOKENSERVER__DATABASE_URL) RUST_TEST_THREADS=1 cargo test
