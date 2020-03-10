##
# Collection of helper scripts used for local dev.
##

SYNC_DATABASE_URL = 'mysql://sample_user:sample_password@localhost/syncstorage_rs'

# This key can live anywhere on your machine. Adjust path as needed.
PATH_TO_SYNC_SPANNER_KEYS = `pwd`/service-account.json

# TODO: replace with rust grpc alternative when ready
# Assumes you've cloned the server-syncstorage repo locally into a peer dir.
# https://github.com/mozilla-services/server-syncstorage
PATH_TO_GRPC_CERT = ../server-syncstorage/local/lib/python2.7/site-packages/grpc/_cython/_credentials/roots.pem

clippy:
	# Matches what's run in circleci
	cargo clippy --all --all-targets -- -D warnings

docker_start:
	docker-compose up -d

docker_start_rebuild:
	docker-compose up --build -d

docker_stop:
	docker-compose down

run:
	RUST_LOG=debug RUST_BACKTRACE=full cargo run -- --config config/local.toml

run_spanner:
	GOOGLE_APPLICATION_CREDENTIALS=$(PATH_TO_SYNC_SPANNER_KEYS) GRPC_DEFAULT_SSL_ROOTS_FILE_PATH=$(PATH_TO_GRPC_CERT) make run

test:
	SYNC_DATABASE_URL=$(SYNC_DATABASE_URL) RUST_TEST_THREADS=1 cargo test
