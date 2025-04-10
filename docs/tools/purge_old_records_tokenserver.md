# Documentation for `purge_old_records.py`

## Summary

The `purge_old_records.py` script is an administrative utility for managing obsolete user records in Tokenserver. It removes outdated user records from the database and deletes associated data from storage nodes. This process helps reduce storage overhead, improve database performance, and maintain the health of the Tokenserver system.

Obsolete records are those replaced by newer records for the same user or marked for deletion if the user has deleted their account. The script can run in batch mode for periodic cleanup and includes options for dry-run testing and forced purging when nodes are down.

---

## Status
    - Running as Kubernetes Workload as part of deployment in `sync-prod` as: `tokenserver-prod-sync-app-1-purge-old-records-0`
    - See YAML configuration when editing each job.
    - See Kubernetes Engine Workload Panel in [`sync-prod`](https://console.cloud.google.com/kubernetes/workload/overview?inv=1&invt=AbmJeQ&project=moz-fx-sync-prod-3f0c) for more information. 

---

## Specifics

- **Primary Functionality**:
  - Deletes obsolete user records.
  - Issues delete requests to user storage nodes to purge related data.
- **Optional Administrative Task**:
  - The script complements internal record replacement handled by the backend but is not mandatory for system operation.
- **Batch Processing**:
  - Operates in loops, processing records in batches of a configurable size.
- **Grace Period**:
  - Provides a grace period to avoid prematurely deleting recently replaced records.
- **Dry Run**:
  - Offers a non-destructive mode for testing.

---

## Notes
- **Regular Use**:
  - Running this script regularly can help maintain system performance and reduce storage overhead.
- **Concurrency**:
  - When running multiple instances of this script, use the `--max-offset` option to reduce collisions.
- **Forced Deletion**:
  - Use the `--force` option cautiously, especially if storage nodes are down.

---

## Instructions for Running the Script

### Prerequisites

1. **Python Environment**: Ensure Python 3.7+ is installed.
2. **Dependencies**:
   - Install required Python packages:
     ```
     pip install requests hawkauthlib backoff
     ```
3. **Configuration**:
   - Set up access to the Tokenserver database.
   - Provide necessary metrics and logging configurations.

### Usage

Run the script using the following command:
```bash
python purge_old_records.py [options] secret
```


### Options

| Option                  | Description                                                                                     | Default      |
|-------------------------|-------------------------------------------------------------------------------------------------|--------------|
| `--purge-interval`      | Interval in seconds to sleep between purging runs.                                             | `3600`       |
| `--grace-period`        | Grace period in seconds before deleting records.                                               | `86400`      |
| `--max-per-loop`        | Maximum number of items to process in each batch.                                              | `10`         |
| `--max-offset`          | Random offset to select records for purging, reducing collisions in concurrent tasks.           | `0`          |
| `--max-records`         | Maximum number of records to purge before exiting.                                             | `0` (no limit) |
| `--request-timeout`     | Timeout in seconds for delete requests to storage nodes.                                       | `60`         |
| `--oneshot`             | Perform a single purge run and exit.                                                           | Disabled     |
| `--dryrun`              | Test the script without making destructive changes.                                            | Disabled     |
| `--force`               | Force purging even if the user's storage node is marked as down.                               | Disabled     |
| `--override_node`       | Specify a node to override for deletions if data is copied.                                    | None         |
| `--range_start`         | Start of UID range to process.                                                                 | None         |
| `--range_end`           | End of UID range to process.                                                                   | None         |
| `--human_logs`          | Enable human-readable logs.                                                                    | Disabled     |

### Examples

#### Example 1: Basic Purge
Perform a basic purge of obsolete user records:
```bash
python purge_old_records.py secret_key
```
#### Example 2: Grace Period and Dry Run
Purge records with a 48-hour grace period in dry-run mode:
```bash
python purge_old_records.py --grace-period 172800 --dryrun secret_key
```

#### Example 3: Specify Range and Offset
Purge records within a UID range with a random offset:

```bash
python purge_old_records.py --range_start uid_start --range_end uid_end --max-offset 50 secret_key
```

#### Example 4: Force Purge on Downed Nodes
Force the deletion of data on downed nodes:
```bash
python purge_old_records.py --force secret_key
```

---

## Detailed Usage

1. **Batch Processing**:
   - The script processes records in batches defined by the `--max-per-loop` option.
   - Each batch is fetched from the database using random offsets to avoid overlapping with concurrent runs.

2. **Grace Period**:
   - The grace period ensures that recently replaced records are not prematurely deleted.

3. **Storage Node Cleanup**:
   - For each user, the script sends a delete request to their storage node to remove associated data.

4. **Metrics Tracking**:
   - Tracks operations like user record deletions, service data deletions, and errors using metrics integration.

5. **Error Handling**:
   - Uses exponential backoff to retry failed HTTP requests.
   - Detects loops in batch processing and raises exceptions.

6. **Dry Run Mode**:
   - Simulates deletions without modifying the database or storage nodes, useful for testing.

---

