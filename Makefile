##
# Collection of helper scripts used for local dev.
##

# This key can live anywhere on your machine. Adjust path as needed.
PATH_TO_SYNC_SPANNER_KEYS = `pwd`/service-account.json

# TODO: replace with rust grpc alternative when ready
# Assumes you've cloned the server-syncstorage repo locally into a peer dir.
# https://github.com/mozilla-services/server-syncstorage
PATH_TO_GRPC_CERT = ../server-syncstorage/local/lib/python2.7/site-packages/grpc/_cython/_credentials/roots.pem

clippy_mysql:
	# Matches what's run in circleci
	cargo clippy --workspace --all-targets -- -D warnings

clippy_spanner:
	# Matches what's run in circleci
	cargo clippy --workspace --all-targets --no-default-features --features=spanner -- -D warnings

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

run_mysql: python
	PATH="./venv/bin:$(PATH)" \
		# See https://github.com/PyO3/pyo3/issues/1741 for discussion re: why we need to set the
		# below env var
		PYTHONPATH=/Users/edonowitz/Dev/syncstorage-rs/venv/lib/python3.9/site-packages \
		RUST_LOG=debug \
		RUST_BACKTRACE=full \
		cargo run -- --config config/local.toml

run_spanner: python
	GOOGLE_APPLICATION_CREDENTIALS=$(PATH_TO_SYNC_SPANNER_KEYS) \
		GRPC_DEFAULT_SSL_ROOTS_FILE_PATH=$(PATH_TO_GRPC_CERT) \
		# See https://github.com/PyO3/pyo3/issues/1741 for discussion re: why we need to set the
		# below env var
		PYTHONPATH=/Users/edonowitz/Dev/syncstorage-rs/venv/lib/python3.9/site-packages \
	    PATH="./venv/bin:$(PATH)" \
		RUST_LOG=debug \
		RUST_BACKTRACE=full \
		cargo run --no-default-features --features=spanner -- --config config/local.toml

test:
	SYNC_SYNCSTORAGE__DATABASE_URL=mysql://sample_user:sample_password@localhost/syncstorage_rs \
		SYNC_TOKENSERVER__DATABASE_URL=mysql://sample_user:sample_password@localhost/tokenserver_rs \
		RUST_TEST_THREADS=1 \
		cargo test
