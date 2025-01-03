##
# Collection of helper scripts used for local dev.
##

# This key can live anywhere on your machine. Adjust path as needed.
PATH_TO_SYNC_SPANNER_KEYS = `pwd`/service-account.json

# TODO: replace with rust grpc alternative when ready
# Assumes you've cloned the server-syncstorage repo locally into a peer dir.
# https://github.com/mozilla-services/server-syncstorage
PATH_TO_GRPC_CERT = ../server-syncstorage/local/lib/python2.7/site-packages/grpc/_cython/_credentials/roots.pem

SRC_ROOT = $(shell pwd)
PYTHON_SITE_PACKGES = $(shell $(SRC_ROOT)/venv/bin/python -c "from distutils.sysconfig import get_python_lib; print(get_python_lib())")

clippy_sqlite:
	# Matches what's run in circleci
	cargo clippy --workspace --all-targets --no-default-features --features=sqlite,py_verifier -- -D warnings

clippy_mysql:
	# Matches what's run in circleci
	cargo clippy --workspace --all-targets --no-default-features --features=mysql,py_verifier -- -D warnings

clippy_spanner:
	# Matches what's run in circleci
	cargo clippy --workspace --all-targets --no-default-features --features=spanner,py_verifier  -- -D warnings

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
		PYTHONPATH=$(PYTHON_SITE_PACKGES) \
		RUST_LOG=debug \
		RUST_BACKTRACE=full \
		cargo run --no-default-features --features=mysql,py_verifier -- --config config/local.toml

run_sqlite: python
	PATH="./venv/bin:$(PATH)" \
		# See https://github.com/PyO3/pyo3/issues/1741 for discussion re: why we need to set the
		# below env var
		PYTHONPATH=$(PYTHON_SITE_PACKGES) \
		RUST_LOG=debug \
		RUST_BACKTRACE=full \
		cargo run --no-default-features --features=sqlite,py_verifier -- --config config/local.toml

run_spanner: python
	GOOGLE_APPLICATION_CREDENTIALS=$(PATH_TO_SYNC_SPANNER_KEYS) \
		GRPC_DEFAULT_SSL_ROOTS_FILE_PATH=$(PATH_TO_GRPC_CERT) \
		# See https://github.com/PyO3/pyo3/issues/1741 for discussion re: why we need to set the
		# below env var
		PYTHONPATH=$(PYTHON_SITE_PACKGES) \
		PATH="./venv/bin:$(PATH)" \
		RUST_LOG=debug \
		RUST_BACKTRACE=full \
		cargo run --no-default-features --features=spanner,py_verifier -- --config config/local.toml

test_mysql:
	SYNC_SYNCSTORAGE__DATABASE_URL=mysql://sample_user:sample_password@localhost/syncstorage_rs \
		SYNC_TOKENSERVER__DATABASE_URL=mysql://sample_user:sample_password@localhost/tokenserver_rs \
		RUST_TEST_THREADS=1 \
		cargo test --workspace --no-default-features --features=mysql,py_verifier

test_sqlite:
	SYNC_SYNCSTORAGE__DATABASE_URL=sqlite:///tmp/syncstorage.db\
		SYNC_TOKENSERVER__DATABASE_URL=sqlite:///tmp/tokenserver.db \
		RUST_TEST_THREADS=1 \
		cargo test --workspace --no-default-features --features=sqlite,py_verifier
