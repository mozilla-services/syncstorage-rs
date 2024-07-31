##
# Collection of helper scripts used for local dev.
##

# This key can live anywhere on your machine. Adjust path as needed.
PATH_TO_SYNC_SPANNER_KEYS = `pwd`/service-account.json

# TODO: replace with rust grpc alternative when ready
# Assumes you've cloned the server-syncstorage repo locally into a peer dir.
# https://github.com/mozilla-services/server-syncstorage
PATH_TO_GRPC_CERT = ../server-syncstorage/local/lib/python2.7/site-packages/grpc/_cython/_credentials/roots.pem

POETRY := $(shell command -v poetry 2> /dev/null)
INSTALL_STAMP := .install.stamp
TOOLS_DIR := tools
FLAKE8_CONFIG := .flake8
PROJECT_ROOT_DIR := ./
ROOT_PYPROJECT_TOML := pyproject.toml
HAWK_DIR := $(TOOLS_DIR)/hawk
INTEGRATION_TEST_DIR := $(TOOLS_DIR)/integration_tests
INTEGRATION_TEST_DIR_TOKENSERVER := $(TOOLS_DIR)/integration_tests/tokenserver
SPANNER_DIR := $(TOOLS_DIR)/spanner
TOKENSERVER_UTIL_DIR := $(TOOLS_DIR)/tokenserver
LOAD_TEST_DIR := $(TOOLS_DIR)/tokenserver/loadtests

# In order to be consumed by the ETE Test Metric Pipeline, files need to follow a strict naming
# convention: {job_number}__{utc_epoch_datetime}__{workflow}__{test_suite}__results{-index}.xml
# TODO: update workflow name appropriately
WORKFLOW := build-deploy
EPOCH_TIME := $(shell date +"%s")
TEST_RESULTS_DIR ?= workflow/test-results
TEST_PROFILE := $(if $(CIRCLECI),ci,default)
TEST_FILE_PREFIX := $(if $(CIRCLECI),$(CIRCLE_BUILD_NUM)__$(EPOCH_TIME)__$(CIRCLE_PROJECT_REPONAME)__$(WORKFLOW)__)
UNIT_JUNIT_XML := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)unit__results.xml
UNIT_COVERAGE_JSON := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)unit__coverage.json
INTEGRATION_JUNIT_XML := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)integration__results.xml
SYNC_SYNCSTORAGE__DATABASE_URL ?= mysql://sample_user:sample_password@localhost/syncstorage_rs
SYNC_TOKENSERVER__DATABASE_URL ?= mysql://sample_user:sample_password@localhost/tokenserver_rs

SRC_ROOT = $(shell pwd)
PYTHON_SITE_PACKGES = $(shell $(SRC_ROOT)/venv/bin/python -c "from distutils.sysconfig import get_python_lib; print(get_python_lib())")

.PHONY: install
install: $(INSTALL_STAMP)  ##  Install dependencies with poetry
$(INSTALL_STAMP): pyproject.toml poetry.lock
	@if [ -z $(POETRY) ]; then echo "Poetry could not be found. See https://python-poetry.org/docs/"; exit 2; fi
	$(POETRY) install
	touch $(INSTALL_STAMP)

hawk:
	# install dependencies for hawk token utility.
	$(POETRY) -V
	$(POETRY) install --directory=$(HAWK_DIR) --no-root

integration-test:
	# install dependencies for integration tests.
	$(POETRY) -V
	$(POETRY) install --directory=$(INTEGRATION_TEST_DIR) --no-root

spanner:
	# install dependencies for spanner utilities.
	$(POETRY) -V
	$(POETRY) install --directory=$(SPANNER_DIR) --no-root

tokenserver:
	# install dependencies for tokenserver utilities.
	$(POETRY) -V
	$(POETRY) install --directory=$(TOKENSERVER_UTIL_DIR) --no-root

tokenserver-load:
	# install dependencies for tokenserver utilities.
	$(POETRY) -V
	$(POETRY) install --directory=$(LOAD_TEST_DIR) --no-root

clippy_mysql:
	# Matches what's run in circleci
	cargo clippy --workspace --all-targets --no-default-features --features=syncstorage-db/mysql --features=py_verifier -- -D warnings

clippy_spanner:
	# Matches what's run in circleci
	cargo clippy --workspace --all-targets --no-default-features --features=syncstorage-db/spanner --features=py_verifier -- -D warnings

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
		cargo run --no-default-features --features=syncstorage-db/mysql --features=py_verifier -- --config config/local.toml

run_spanner: python
	GOOGLE_APPLICATION_CREDENTIALS=$(PATH_TO_SYNC_SPANNER_KEYS) \
		GRPC_DEFAULT_SSL_ROOTS_FILE_PATH=$(PATH_TO_GRPC_CERT) \
		# See https://github.com/PyO3/pyo3/issues/1741 for discussion re: why we need to set the
		# below env var
		PYTHONPATH=$(PYTHON_SITE_PACKGES) \
	    PATH="./venv/bin:$(PATH)" \
		RUST_LOG=debug \
		RUST_BACKTRACE=full \
		cargo run --no-default-features --features=syncstorage-db/spanner --features=py_verifier -- --config config/local.toml

.ONESHELL:
test:
	SYNC_SYNCSTORAGE__DATABASE_URL=${SYNC_SYNCSTORAGE__DATABASE_URL} \
	SYNC_TOKENSERVER__DATABASE_URL=${SYNC_TOKENSERVER__DATABASE_URL} \
	RUST_TEST_THREADS=1 \
	cargo nextest run --workspace --profile ${TEST_PROFILE} $(ARGS)

.ONESHELL:
test_with_coverage:
	SYNC_SYNCSTORAGE__DATABASE_URL=${SYNC_SYNCSTORAGE__DATABASE_URL} \
	SYNC_TOKENSERVER__DATABASE_URL=${SYNC_TOKENSERVER__DATABASE_URL} \
	RUST_TEST_THREADS=1 \
	cargo llvm-cov --no-report --summary-only \
		nextest --workspace --profile ${TEST_PROFILE}; exit_code=$$?
	mv target/nextest/${TEST_PROFILE}/junit.xml ${UNIT_JUNIT_XML}
	exit $$exit_code

merge_coverage_results:
	cargo llvm-cov report --summary-only --json --output-path ${UNIT_COVERAGE_JSON}

.ONESHELL:
run_token_server_integration_tests:
	pip3 install -r tools/tokenserver/requirements.txt
	pytest tools/tokenserver --junit-xml=${INTEGRATION_JUNIT_XML}
	SYNC_SYNCSTORAGE__DATABASE_URL=mysql://sample_user:sample_password@localhost/syncstorage_rs \
		SYNC_TOKENSERVER__DATABASE_URL=mysql://sample_user:sample_password@localhost/tokenserver_rs \
		RUST_TEST_THREADS=1 \
		cargo test --workspace

.PHONY: isort
isort: $(INSTALL_STAMP)  ##  Run isort
	$(POETRY) run isort --check-only $(ROOT_PYPROJECT_TOML) $(PROJECT_ROOT_DIR)

.PHONY: black
black: $(INSTALL_STAMP)  ##  Run black
	$(POETRY) run black --quiet --diff --check $(ROOT_PYPROJECT_TOML) $(PROJECT_ROOT_DIR)

.PHONY: format
format: $(INSTALL_STAMP)  ##  Sort imports and reformats code
	$(POETRY) run isort $(ROOT_PYPROJECT_TOML) $(PROJECT_ROOT_DIR)
	$(POETRY) run black $(ROOT_PYPROJECT_TOML) $(PROJECT_ROOT_DIR)

.PHONY: flake8
flake8: $(INSTALL_STAMP)  ##  Run flake8
	$(POETRY) run flake8 --config $(FLAKE8_CONFIG) $(PROJECT_ROOT_DIR)
 
.PHONY: bandit
bandit: $(INSTALL_STAMP)  ##  Run bandit
	$(POETRY) run bandit --quiet -r -c $(ROOT_PYPROJECT_TOML) $(PROJECT_ROOT_DIR)

.PHONY: mypy
mypy: $(INSTALL_STAMP)  ##  Run mypy
	$(POETRY) run mypy --config-file=$(ROOT_PYPROJECT_TOML) $(PROJECT_ROOT_DIR)
 
.PHONY: pydocstyle
pydocstyle: $(INSTALL_STAMP)  ##  Run pydocstyle
	$(POETRY) run pydocstyle -es --count --config=$(ROOT_PYPROJECT_TOML) $(PROJECT_ROOT_DIR)
