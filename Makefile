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
PROJECT_ROOT_DIR := ./
ROOT_PYPROJECT_TOML := pyproject.toml
HAWK_DIR := $(TOOLS_DIR)/hawk
INTEGRATION_TEST_DIR := $(TOOLS_DIR)/integration_tests
INTEGRATION_TEST_DIR_TOKENSERVER := $(TOOLS_DIR)/integration_tests/tokenserver
SPANNER_DIR := $(TOOLS_DIR)/spanner
TOKENSERVER_UTIL_DIR := $(TOOLS_DIR)/tokenserver
LOAD_TEST_DIR := $(TOOLS_DIR)/tokenserver/loadtests
SYNCSTORAGE_LOAD_TEST_DIR := $(TOOLS_DIR)/syncstorage-loadtest
RUST_LOG ?= debug

# In order to be consumed by the ETE Test Metric Pipeline, files need to follow a strict naming
# convention: {job_number}__{utc_epoch_datetime}__{workflow}__{test_suite}__results{-index}.xml
# TODO: update workflow name appropriately
WORKFLOW := build-deploy
EPOCH_TIME := $(shell date +"%s")
TEST_RESULTS_DIR ?= workflow/test-results
TEST_PROFILE := $(if $(or $(CIRCLECI),$(GITHUB_ACTIONS)),ci,default)
TEST_FILE_PREFIX := $(if $(GITHUB_ACTIONS),$(GITHUB_RUN_NUMBER)__$(EPOCH_TIME)__$(notdir $(GITHUB_REPOSITORY))__$(WORKFLOW)__)
UNIT_JUNIT_XML := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)unit__results.xml
MYSQL_UNIT_JUNIT_XML := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)mysql_unit__results.xml
POSTGRES_UNIT_JUNIT_XML := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)postgres_unit__results.xml
SPANNER_UNIT_JUNIT_XML := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)spanner_unit__results.xml
MYSQL_COVERAGE_JSON := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)mysql_unit__coverage.json
POSTGRES_COVERAGE_JSON := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)postgres_unit__coverage.json
SPANNER_COVERAGE_JSON := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)spanner_unit__coverage.json
UNIT_COVERAGE_JSON := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)unit__coverage.json

SPANNER_INT_JUNIT_XML := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)spanner_integration__results.xml
SPANNER_NO_JWK_INT_JUNIT_XML := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)spanner_no_oauth_integration__results.xml
POSTGRES_INT_JUNIT_XML := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)postgres_integration__results.xml
POSTGRES_NO_JWK_INT_JUNIT_XML := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)postgres_no_oauth_integration__results.xml
MYSQL_INT_JUNIT_XML := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)mysql_integration__results.xml
MYSQL_NO_JWK_INT_JUNIT_XML := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)mysql_no_oauth_integration__results.xml

LOCAL_INTEGRATION_JUNIT_XML := $(TEST_RESULTS_DIR)/$(TEST_FILE_PREFIX)local_integration__results.xml
SYNC_SYNCSTORAGE__DATABASE_URL ?= mysql://sample_user:sample_password@localhost/syncstorage_rs
SYNC_TOKENSERVER__DATABASE_URL ?= mysql://sample_user:sample_password@localhost/tokenserver_rs

SRC_ROOT = $(shell pwd)
PYTHON_SITE_PACKAGES = $(shell poetry run python -c "from distutils.sysconfig import get_python_lib; print(get_python_lib())")

clippy_mysql:
	# Matches what's run in circleci
	cargo clippy --workspace --all-targets --no-default-features --features=syncstorage-db/mysql --features=py_verifier -- -D clippy::dbg_macro -D warnings

clippy_postgres:
	cargo clippy --workspace --all-targets --no-default-features --features=syncstorage-db/postgres --features=py_verifier -- -D clippy::dbg_macro -D warnings

clippy_spanner:
	# Matches what's run in circleci
	cargo clippy --workspace --all-targets --no-default-features --features=syncstorage-db/spanner --features=py_verifier -- -D clippy::dbg_macro -D warnings

clean:
	cargo clean

docker_start_mysql:
	docker compose -f docker-compose.mysql.yaml up -d

docker_start_mysql_rebuild:
	docker compose -f docker-compose.mysql.yaml up --build -d

docker_stop_mysql:
	docker compose -f docker-compose.mysql.yaml down

docker_start_spanner:
	docker compose -f docker-compose.spanner.yaml up -d

docker_start_spanner_rebuild:
	docker compose -f docker-compose.spanner.yaml up --build -d

docker_stop_spanner:
	docker compose -f docker-compose.spanner.yaml down

.ONESHELL:
docker_run_mysql_e2e_tests:
	docker compose \
		-f docker-compose.mysql.yaml \
		-f docker-compose.e2e.mysql.yaml \
	 	up \
	 	--exit-code-from mysql-e2e-tests \
	 	--abort-on-container-exit;
	exit_code=$$?;
	docker cp mysql-e2e-tests:/mysql_integration_results.xml ${MYSQL_INT_JUNIT_XML};
	docker cp mysql-e2e-tests:/mysql_no_jwk_integration_results.xml ${MYSQL_NO_JWK_INT_JUNIT_XML};
	exit $$exit_code;

.ONESHELL:
docker_run_postgres_e2e_tests:
	docker compose \
		-f docker-compose.postgres.yaml \
		-f docker-compose.e2e.postgres.yaml \
	 	up \
	 	--exit-code-from postgres-e2e-tests \
	 	--abort-on-container-exit;
	exit_code=$$?;
	docker cp postgres-e2e-tests:/postgres_integration_results.xml ${POSTGRES_INT_JUNIT_XML};
	docker cp postgres-e2e-tests:/postgres_no_jwk_integration_results.xml ${POSTGRES_NO_JWK_INT_JUNIT_XML};
	exit $$exit_code;

.ONESHELL:
docker_run_spanner_e2e_tests:
	docker compose \
		-f docker-compose.spanner.yaml \
		-f docker-compose.e2e.spanner.yaml \
	 	up \
	 	--exit-code-from spanner-e2e-tests \
	 	--abort-on-container-exit;
	exit_code=$$?;
	docker cp spanner-e2e-tests:/spanner_integration_results.xml ${SPANNER_INT_JUNIT_XML};
	docker cp spanner-e2e-tests:/spanner_no_jwk_integration_results.xml ${SPANNER_NO_JWK_INT_JUNIT_XML};
	exit $$exit_code;

run_mysql: $(INSTALL_STAMP)
	# See https://github.com/PyO3/pyo3/issues/1741 for discussion re: why we need to set the
	# below env var
	PYTHONPATH=$(PYTHON_SITE_PACKAGES) \
	        RUST_LOG=$(RUST_LOG) \
		RUST_BACKTRACE=full \
		cargo run --no-default-features --features=syncstorage-db/mysql --features=py_verifier -- --config config/local.toml

run_spanner: $(INSTALL_STAMP)
	GOOGLE_APPLICATION_CREDENTIALS=$(PATH_TO_SYNC_SPANNER_KEYS) \
		GRPC_DEFAULT_SSL_ROOTS_FILE_PATH=$(PATH_TO_GRPC_CERT) \
		# See https://github.com/PyO3/pyo3/issues/1741 for discussion re: why we need to set the
		# below env var
		PYTHONPATH=$(PYTHON_SITE_PACKAGES) \
		RUST_LOG=$(RUST_LOG) \
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
	cargo llvm-cov --summary-only --json --output-path ${MYSQL_COVERAGE_JSON} \
		nextest --workspace --profile ${TEST_PROFILE}; exit_code=$$?
	mv target/nextest/${TEST_PROFILE}/junit.xml ${MYSQL_UNIT_JUNIT_XML}
	exit $$exit_code

.ONESHELL:
spanner_test_with_coverage:
	cargo llvm-cov --summary-only --json --output-path ${SPANNER_COVERAGE_JSON} \
		nextest --workspace --no-default-features --features=syncstorage-db/spanner --features=py_verifier --profile ${TEST_PROFILE}; exit_code=$$?
	mv target/nextest/${TEST_PROFILE}/junit.xml ${SPANNER_UNIT_JUNIT_XML}
	exit $$exit_code

.ONESHELL:
postgres_test_with_coverage:
	cargo llvm-cov --summary-only --json --output-path ${POSTGRES_COVERAGE_JSON} \
		nextest --workspace --no-default-features --features=syncstorage-db/postgres --features=tokenserver-db/postgres --features=py_verifier --profile ${TEST_PROFILE}; exit_code=$$?
	mv target/nextest/${TEST_PROFILE}/junit.xml ${POSTGRES_UNIT_JUNIT_XML}
	exit $$exit_code

.ONESHELL:
run_token_server_integration_tests:
	cd tools/tokenserver
	poetry install --no-root --without dev
	poetry run pytest tools/tokenserver --junit-xml=${INTEGRATION_JUNIT_XML}

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
	# install dependencies for tokenserver load tests.
	$(POETRY) -V
	$(POETRY) install --directory=$(LOAD_TEST_DIR) --no-root

## Syncstorage Load Tests
syncstorage-loadtest:
	# install dependencies for syncstorage load tests.
	@echo "Installing syncstorage load test dependencies with Python 3.10+"
	@cd $(SYNCSTORAGE_LOAD_TEST_DIR) && \
		eval "$$(pyenv init -)" && \
		eval "$$(pyenv virtualenv-init -)" && \
		python --version && \
		$(POETRY) env use python && \
		$(POETRY) install --no-root

.PHONY: loadtest-install
loadtest-install: syncstorage-loadtest  ##  Install dependencies for syncstorage load tests.

.PHONY: loadtest-direct
loadtest-direct: syncstorage-loadtest  ##  Run load tests with direct access mode (requires SERVER_URL with secret).
	@echo "Running syncstorage load tests in direct access mode..."
	@echo "Usage: make loadtest-direct SERVER_URL='http://localhost:8000#secretValue' [MOLOTOV_ARGS='--max-runs 5']"
	cd $(SYNCSTORAGE_LOAD_TEST_DIR) && \
	SERVER_URL=$(or $(SERVER_URL),"http://localhost:8000#changeme") \
	$(POETRY) run molotov $(or $(MOLOTOV_ARGS),--max-runs 5 -cxv) loadtest.py

.PHONY: loadtest-fxa
loadtest-fxa: syncstorage-loadtest  ##  Run load tests with FxA OAuth mode (Stage environment).
	@echo "Running syncstorage load tests with FxA OAuth..."
	@echo "Usage: make loadtest-fxa [SERVER_URL=...] [MOLOTOV_ARGS='--workers 3 --duration 60']"
	cd $(SYNCSTORAGE_LOAD_TEST_DIR) && \
	SERVER_URL=$(or $(SERVER_URL),"https://token.stage.mozaws.net") \
	FXA_API_HOST=$(or $(FXA_API_HOST),"https://api-accounts.stage.mozaws.net") \
	FXA_OAUTH_HOST=$(or $(FXA_OAUTH_HOST),"https://oauth.stage.mozaws.net") \
	$(POETRY) run molotov $(or $(MOLOTOV_ARGS),--workers 3 --duration 60 -v) loadtest.py

.PHONY: loadtest-jwt
loadtest-jwt: syncstorage-loadtest  ##  Run load tests with self-signed JWT mode (requires OAUTH_PRIVATE_KEY_FILE).
	@echo "Running syncstorage load tests with self-signed JWTs..."
	@echo "Usage: make loadtest-jwt OAUTH_PRIVATE_KEY_FILE=/path/to/key.pem [SERVER_URL=...] [MOLOTOV_ARGS='--workers 100 --duration 300']"
	@if [ -z "$(OAUTH_PRIVATE_KEY_FILE)" ]; then \
		echo "Error: OAUTH_PRIVATE_KEY_FILE is required"; \
		echo "Example: make loadtest-jwt OAUTH_PRIVATE_KEY_FILE=/path/to/load_test.pem"; \
		exit 1; \
	fi
	cd $(SYNCSTORAGE_LOAD_TEST_DIR) && \
	SERVER_URL=$(or $(SERVER_URL),"http://localhost:8000") \
	OAUTH_PRIVATE_KEY_FILE=$(OAUTH_PRIVATE_KEY_FILE) \
	$(POETRY) run molotov $(or $(MOLOTOV_ARGS),--workers 100 --duration 300 -v) loadtest.py

.PHONY: loadtest-docker
loadtest-docker:  ##  Run load tests in Docker container.
	@echo "Running syncstorage load tests in Docker..."
	docker run -e TEST_REPO=https://github.com/mozilla-services/syncstorage-loadtest -e TEST_NAME=test tarekziade/molotov:latest

## Python Utilities
.PHONY: ruff-lint
ruff-lint: $(INSTALL_STAMP)  ##  Lint check for utilities.
	$(POETRY) run ruff check $(TOOLS_DIR)

.PHONY: ruff-fmt-chk
ruff-fmt-chk: $(INSTALL_STAMP)  ##  Format check with change summary.
	$(POETRY) run ruff format --diff  $(TOOLS_DIR)

.PHONY: ruff-format
ruff-format: $(INSTALL_STAMP)  ##  Formats files in directory.
	$(POETRY) run ruff format $(TOOLS_DIR)

.PHONY: py-deps-latest
py-deps-latest: $(INSTALL_STAMP)  ##  Checks latest versions in PyPI
	$(POETRY) show --latest --top-level $(TOOLS_DIR)

.PHONY: py-deps-outdated
py-deps-outdated: $(INSTALL_STAMP)  ##  Checks for outdated Python packages
	$(POETRY) show --outdated $(TOOLS_DIR)

# Documentation utilities
.PHONY: doc-install-deps
doc-install-deps:  ## Install the dependencies for doc generation
	cargo install mdbook && cargo install mdbook-mermaid && mdbook-mermaid install docs/

.PHONY: doc-test
doc-test:  ##  Tests documentation for errors.
	mdbook test docs/

.PHONY: doc-clean
doc-clean:  ##  Erases output/ contents and clears mdBook output.
	mdbook clean docs/

.PHONY: doc-watch
doc-watch:  ##  Generate live preview of docs and open in browser and watch. No build artifacts.
	mdbook clean docs/
	mdbook watch docs/ --open

.PHONY: doc-prev
doc-prev:  ##  Generate live preview of docs and open in browser.
	mdbook-mermaid install docs/
	mdbook clean docs/
	mdbook build docs/
	mdbook serve docs/ --open


SWAGGER_IMG := swaggerapi/swagger-ui
SWAGGER_NAME := swagger-ui-preview
OPENAPI_FILE := openapi.json

.PHONY: api-prev
api-prev: ## Generate live preview of OpenAPI Swagger Docs and open in browser (port 8080).
	@set -e; \
	echo "Generating OpenAPI spec..."; \
	cargo run --example generate_openapi_spec > $(OPENAPI_FILE); \
	echo "Starting Swagger UI..."; \
	cid="$$(docker ps -q -f name=^/$(SWAGGER_NAME)$$)"; \
	if [ -n "$$cid" ]; then \
	  echo "Restarting existing container $$cid"; \
	  docker restart "$$cid" >/dev/null; \
	else \
	  old="$$(docker ps -aq -f name=^/$(SWAGGER_NAME)$$)"; \
	  if [ -n "$$old" ]; then \
	    echo "Removing stale container $$old"; \
	    docker rm -f "$$old" >/dev/null || true; \
	  fi; \
	  cid="$$(docker run -d --rm \
	    --name $(SWAGGER_NAME) \
	    -p 8080:8080 \
	    -e SWAGGER_JSON=/openapi.json \
	    -v $$(pwd)/$(OPENAPI_FILE):/openapi.json:ro \
	    $(SWAGGER_IMG))"; \
	  echo "Started container $$cid"; \
	fi; \
	if command -v open >/dev/null 2>&1; then \
	  open http://localhost:8080; \
	elif command -v xdg-open >/dev/null 2>&1; then \
	  xdg-open http://localhost:8080; \
	else \
	  echo "Open http://localhost:8080 manually"; \
	fi; \
	trap 'echo ""; echo "Stopping Swagger UI..."; docker stop "$$cid" >/dev/null || true' INT TERM EXIT; \
	echo "Swagger UI running (Ctrl-C to stop)"; \
	wait
