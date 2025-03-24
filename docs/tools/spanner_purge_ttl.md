# Documentation for `purge_ttl.py`

## Summary

The `purge_ttl.py` script is a utility for purging expired Time-To-Live (TTL) records from a Google Spanner database. This script is designed to manage and clean up old data from specific database tables, ensuring efficient use of storage and maintaining database performance. It offers flexible options for targeting specific collections, user ID prefixes, and modes of operation, with optional dry-run functionality for testing changes without affecting the database.

---

## Status
    - Running as Kubernetes Workload cron job in `sync-prod`.
    - Runs at 10 minutes past every 2nd hour.
    - Runs per-collection and is configured for each of the following: 
        - batches
        - clients
        - crypto
        - forms
        - meta
        - tabs
    - See YAML configuration when editing each job.
    - See Kubernetes Engine Workload Panel in [`sync-prod`](https://console.cloud.google.com/kubernetes/workload/overview?inv=1&invt=AbmJeQ&project=moz-fx-sync-prod-3f0c) for more information. 

## Specifics

- **Database**: Google Spanner.
- **Tables**: 
  - `batches`: Contains batch entries, with cascading deletes for child `batch_bsos`.
  - `bsos`: Stores Sync Basic Storage Objects (BSO).
- **Supported Modes**:
  - `batches`: Purges expired entries in the `batches` table.
  - `bsos`: Purges expired entries in the `bsos` table.
  - `both`: Performs purges on both tables.
- **Expiry Modes**:
  - `now`: Purges entries with `expiry < CURRENT_TIMESTAMP()`.
  - `midnight`: Purges entries with `expiry < TIMESTAMP_TRUNC(CURRENT_TIMESTAMP(), DAY, "UTC")`.

The script uses parameters like collection IDs, user ID prefixes, and auto-splitting for fine-grained control over the purging process. It tracks execution duration and results using StatsD metrics for performance monitoring.

---

## Notes

- Ensure proper access to the Spanner instance and database through IAM permissions.
- Use the `--dryrun` option to verify query logic before actual purging.
- Consider setting up automated monitoring for long-running operations or performance issues.

---

## Instructions for Running the Script

### Prerequisites

1. **Python Environment**: Ensure Python 3.7+ is installed.
2. **Google Cloud SDK**: Install and authenticate with Google Cloud.
3. **Dependencies**: Install required Python packages:
   ```bash
   pip install google-cloud-spanner statsd
   ```
4. **Environment Variables:**
    `INSTANCE_ID`: Spanner instance ID (default: spanner-test).
    `DATABASE_ID`: Database ID (default: sync_schema3).
    `SYNC_SYNCSTORAGE__DATABASE_URL`: Database connection URL (e.g., spanner://instance/database).

### Usage

Run the script using the following command:
   ```bash
   python purge_ttl.py [options]
   ```

#### Options

| Option                          | Description                                                                                                     | Default                      |
|---------------------------------|-----------------------------------------------------------------------------------------------------------------|------------------------------|
| `-i`, `--instance_id`           | Spanner instance ID.                                                                                           | `spanner-test`              |
| `-d`, `--database_id`           | Spanner database ID.                                                                                           | `sync_schema3`              |
| `-u`, `--sync_database_url`     | Spanner DSN connection URL (overrides `instance_id` and `database_id`).                                        | `SYNC_SYNCSTORAGE__DATABASE_URL` |
| `--collection_ids`, `--ids`     | Comma-separated list of collection IDs to purge.                                                               | `[]`                        |
| `--uid_prefixes`, `--prefix`    | Comma-separated list of UID prefixes to filter purges.                                                         | `[]`                        |
| `--auto_split`                  | Automatically generate UID prefixes for the specified number of hexadecimal digits.                            | None                        |
| `--mode`                        | Purge mode: `batches`, `bsos`, or `both`.                                                                      | `both`                      |
| `--expiry_mode`                 | Expiry mode: `now` (current timestamp) or `midnight` (start of current day, UTC).                              | `midnight`                  |
| `--dryrun`                      | Perform a dry run without making changes to the database.                                                      | `False`                     |

#### Examples

##### Example 1: Basic Purge
Purge expired entries from both `batches` and `bsos` tables using default configurations:
```bash
    python purge_ttl.py
```

##### Example 2: Specify Instance and Database
Purge expired entries in a specific instance and database:
```bash
    python purge_ttl.py -i my-instance -d my-database
```
##### Example 3: Filter by Collection IDs
Purge only for specific collection IDs:
```bash
    python purge_ttl.py --collection_ids [123,456,789]
```
##### Example 4: Filter by UID Prefixes
Limit purging to specific UID prefixes:
```bash
    python purge_ttl.py --uid_prefixes [abc,def,123]
```
##### Example 5: Auto-Generated Prefixes
Generate prefixes automatically for a 2-digit hexadecimal range:
```bash
    python purge_ttl.py --auto_split 2
```
##### Example 6: Perform a Dry Run
Test the script without making actual changes:
```bash
    python purge_ttl.py --dryrun
```

### Detailed Usage

1. **Connecting to Spanner**:
   - The script connects to Google Spanner using either explicitly provided `instance_id` and `database_id` or a DSN URL.

2. **Purge Modes**:
   - `batches`: Deletes expired entries from the `batches` table, which cascades deletions for `batch_bsos` via Spanner's `ON DELETE CASCADE`.
   - `bsos`: Deletes expired Binary Sync Objects (BSOs).
   - `both`: Executes purges on both `batches` and `bsos`.

3. **Expiry Conditions**:
   - `now`: Purge entries that have already expired at the current timestamp.
   - `midnight`: Purge entries that expired at or before the start of the current UTC day.

4. **Query Customization**:
   - Filters can be added based on collection IDs or UID prefixes.
   - Queries are dynamically constructed using helper functions (`add_conditions`, `get_expiry_condition`).

5. **Performance Monitoring**:
   - Metrics for execution duration and rows affected are logged and sent to StatsD for monitoring.

6. **Error Handling**:
   - The script validates input parameters, raises exceptions for invalid configurations, and logs details for troubleshooting.

7. **Dry Run**:
   - Enabling the `--dryrun` flag ensures that the queries are constructed and logged without executing them on the database.
