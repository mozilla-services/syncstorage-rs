# Summary of Files in tools/Tokenserver

| File Name                       | Description                                                                |
|--------------------------------|-----------------------------------------------------------------------------|
| `add_node.py`                | Adds new node to the tokenserver's database, registering it for user allocation. |
| `allocate_user.py`           | Script to allocate a specific user to a node.                               |
| `conftest.py`                | Pytest configuration and fixtures for shared test setup.                    |
| `count_users.py`             | Script to emit total-user-count metrics for exec dashboard.                 |
| `database.py`                | Shared database utility queries and functions used by multiple scripts.     |
| `purge_old_records.py`       | Script to purge user records that have been replaced.                       |
| `pytest.ini`                 | Configuration file for pytest, specifying options like test output format.  |
| `remove_node.py`             | Script to remove a node from the system.                                    |
| `test_database.py`           | Unit tests for `database.py`.                                               |
| `test_purge_old_records.py`  | Tests for `purge_old_records.py`, validating cleanup operations.            |
| `test_scripts.py`            | Testing module to test the various scripts in this directory.               |
| `unassign_node.py`           | Removes a node from the system and clears any assignments to the named node.| 
| `update_node.py`             | Script to update node status in the db.                                     |

## Unit tests

Install dependencies with `poetry -C tools/tokenserver install --only main`.
Run against a disposable, migrated Tokenserver database (the fixtures delete
users, nodes, and services):

The Make target requires an explicit, nonempty `SYNC_TOKENSERVER__DATABASE_URL`
from the environment or command line; it rejects the Makefile's development
database default. You must still choose a disposable database.

```sh
SYNC_TOKENSERVER__DATABASE_URL=mysql://test:test@127.0.0.1/tokenserver make run_tokenserver_unit_tests
SYNC_TOKENSERVER__DATABASE_URL=postgres://test:test@127.0.0.1/tokenserver make run_tokenserver_unit_tests TOKENSERVER_TEST_BACKEND=postgres
```

CI runs these after the Rust unit tests in the MySQL and PostgreSQL jobs.
The PostgreSQL job applies every migration in filename order, stopping on the first
failure, to initialize the schema for the Python tests because
the Rust tests roll back their schema changes with their test transactions.
Reports go to
`workflow/test-results/*unit__tokenserver-{mysql,postgres}-results.xml` and use
the existing unit report uploads.
