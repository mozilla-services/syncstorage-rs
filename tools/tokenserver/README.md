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