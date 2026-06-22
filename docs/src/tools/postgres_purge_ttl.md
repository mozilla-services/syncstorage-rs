# Documentation for `purge_ttl.py` (PostgreSQL)

## Summary

The `purge_ttl.py` script is a utility for purging expired Time-To-Live (TTL) records from a Sync storage PostgreSQL database. It deletes expired entries from the `batches` and/or `bsos` tables, with optional filtering by collection ID and a dry-run mode for verifying query logic without modifying data.

> Spanner no longer uses this script — it expires records natively via its built-in [row deletion (TTL) policies](https://cloud.google.com/spanner/docs/ttl). This utility applies to PostgreSQL deployments only.

---

## Status
- Runs as a regularly scheduled job (e.g. a cron job) in PostgreSQL ("enterprise") deployments.
- The script is bundled in the main `syncserver-postgres` image and invoked from there; there is no separate utility image.

## Specifics

- **Database**: PostgreSQL (accepts a `postgresql://` or `postgres://` DSN).
- **Tables**:
  - `batches`: Contains batch entries (child `batch_bsos` rows are removed via the schema's foreign-key cascade).
  - `bsos`: Stores Sync Basic Storage Objects (BSO).
- **Supported Modes**:
  - `batches`: Purges expired entries in the `batches` table.
  - `bsos`: Purges expired entries in the `bsos` table.
  - `both`: Performs purges on both tables.
- **Expiry Modes**:
  - `now`: Purges entries with `expiry < CURRENT_TIMESTAMP`.
  - `midnight`: Purges entries with `expiry < DATE_TRUNC('day', CURRENT_TIMESTAMP AT TIME ZONE 'UTC')`.

The script tracks execution duration and rows affected using StatsD metrics for performance monitoring.

---

## Notes

- Ensure the database role used by the DSN has `DELETE` privileges on the target tables.
- Use the `--dryrun` option to verify query logic before actual purging.
- Consider setting up automated monitoring for long-running operations or performance issues.

---

## Instructions for Running the Script

### Prerequisites

1. **Python Environment**: Ensure Python 3 is installed.
2. **Poetry**: Install [Poetry](https://python-poetry.org/docs/#installation).
3. **Dependencies**: From `tools/postgres`, run `poetry install`.
4. **Environment Variables:**
    `SYNC_SYNCSTORAGE__DATABASE_URL`: Database connection URL (e.g., `postgresql://user:pass@host:5432/syncstorage`). Used when `--database_url` is not supplied.

### Usage

Run the script using the following command:
   ```bash
   poetry run python purge_ttl.py [options]
   ```

#### Options

| Option                          | Description                                                                            | Default                          |
|---------------------------------|----------------------------------------------------------------------------------------|----------------------------------|
| `-u`, `--database_url`          | PostgreSQL DSN (`postgresql://...` or `postgres://...`). Required if the env var is unset. | `SYNC_SYNCSTORAGE__DATABASE_URL` |
| `--collection_ids`, `--ids`     | Comma-separated list of collection IDs to purge.                                       | `[]`                             |
| `--mode`                        | Purge mode: `batches`, `bsos`, or `both`.                                              | `both`                           |
| `--expiry_mode`                 | Expiry mode: `now` (current timestamp) or `midnight` (start of current day, UTC).      | `midnight`                       |
| `--dryrun`                      | Perform a dry run without making changes to the database.                              | `False`                          |

#### Examples

##### Example 1: Basic Purge
Purge expired entries from both `batches` and `bsos` tables using the DSN from the environment:
```bash
    poetry run python purge_ttl.py
```

##### Example 2: Specify the Database URL
```bash
    poetry run python purge_ttl.py -u postgresql://user:pass@host:5432/syncstorage
```

##### Example 3: Filter by Collection IDs
Purge only for specific collection IDs:
```bash
    poetry run python purge_ttl.py --collection_ids [123,456,789]
```

##### Example 4: Purge a Single Table
```bash
    poetry run python purge_ttl.py --mode bsos
```

##### Example 5: Perform a Dry Run
Test the script without making actual changes:
```bash
    poetry run python purge_ttl.py --dryrun
```

### Detailed Usage

1. **Connecting to PostgreSQL**:
   - The script builds a SQLAlchemy engine from the supplied DSN. A `postgres://` scheme is normalized to `postgresql://` for newer SQLAlchemy versions.

2. **Purge Modes**:
   - `batches`: Deletes expired entries from the `batches` table.
   - `bsos`: Deletes expired BSOs.
   - `both`: Executes purges on both `batches` and `bsos`.

3. **Expiry Conditions**:
   - `now`: Purge entries that have already expired at the current timestamp.
   - `midnight`: Purge entries that expired at or before the start of the current UTC day.

4. **Query Customization**:
   - An optional collection-ID filter is appended via the `add_conditions` helper using bound parameters.

5. **Performance Monitoring**:
   - Metrics for execution duration and rows affected are logged and sent to StatsD for monitoring.

6. **Dry Run**:
   - Enabling the `--dryrun` flag ensures that the queries are constructed and logged without executing them on the database.
