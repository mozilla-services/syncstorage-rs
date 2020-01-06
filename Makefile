##
# Collection of helper scripts used for local dev.
##

SYNC_DATABASE_URL = 'mysql://sample_user:sample_password@localhost/syncstorage_rs'

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

test:
	cd db-tests && SYNC_DATABASE_URL=$(SYNC_DATABASE_URL) RUST_TEST_THREADS=1 cargo test
