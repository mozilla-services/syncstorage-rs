# Documentation for `process_account_events.py`

## Summary

The `process_account_events.py` is a Python utility designed to handle account-related events for Tokenserver. It connects to an Amazon Simple Queue Service (SQS) queue to poll for events indicating activity on an upstream account, related to user account activities, such as account deletions, password resets, and password changes. These events are processed to maintain synchronization between upstream account actions and Tokenserver's database.

The script is intended for internal use within Mozilla's Firefox Accounts (FxA)-supported deployments and provides a mechanism for administrative tasks like disconnecting devices or marking accounts for cleanup.

Note that this is a purely optional administrative task, highly specific to
Mozilla's internal Firefox-Accounts-supported deployment.

See [FxA Auth Server Docs](https://github.com/mozilla/fxa-auth-server/blob/master/docs/service_notifications.md) for more information on these events.

---

## Status
    - Running as Kubernetes Workload as part of deployment in `sync-prod` as: `tokenserver-prod-sync-app-1-process-account-events`.
    - See YAML configuration when editing configuration.
    - See Kubernetes Engine Workload Panel in [`sync-prod`](https://console.cloud.google.com/kubernetes/workload/overview?inv=1&invt=AbmJeQ&project=moz-fx-sync-prod-3f0c) for more information. 

---

## Supported Event Types

The script processes the following event types:

1. **Delete**  
   - **Event:** Account was deleted.
   - **Description:** Marks user accounts as "retired" to flag them for garbage collection.  
   - **Purpose:** Ensures that deleted accounts are appropriately flagged for eventual cleanup.  
   - **Implementation:** Calls `database.retire_user(email)`.

2. **Reset**  
   - **Event:** Account password was reset.
   - **Description:** Handles password reset events by updating the generation number in the database.  
   - **Purpose:** Disconnects other devices associated with the account.  
   - **Implementation:** Calls `update_generation_number()` with a decremented generation number.

3. **PasswordChange**  
   - **Event:** Account password was changed.
   - **Description:** Processes password change events similarly to reset events by updating the generation number.  
   - **Purpose:** Disconnects other devices to reflect the password change.  
   - **Implementation:** Calls `update_generation_number()` with a decremented generation number.

---

## How It Works

1. **Connects to the SQS Queue:** 
   - Automatically determines the AWS region if not provided.
   - Connects to the specified queue and sets up polling.

2. **Polls for Events:** 
   - Polls indefinitely, waiting for messages on the queue.
   - Processes each event based on its type, using the `process_account_event()` function.

3. **Handles Event Logic:** 
   - Parses the event JSON.
   - Identifies the event type and processes it using specialized logic for each supported event type.

4. **Updates Database:** 
   - Performs necessary database updates, such as retiring accounts or adjusting generation numbers.

5. **Logs and Metrics:** 
   - Logs actions for debugging and administrative purposes.
   - Tracks metrics for processed events using the `metrics` utility.

---

## Notes

- **Optional Administrative Task:** This script is a utility for administrative purposes and is not required for the core functionality of the Syncstorage service.
- **Error Handling:** The script is designed to handle unexpected errors gracefully, logging invalid messages and continuing with the next event.
- **Event Backlog:** Unrecognized event types are logged as warnings and removed from the queue to avoid backlog.

---

## Instructions for Running the Script

### Prerequisites

1. **Python Environment:** Ensure you have Python installed along with the required libraries (`boto`, `json`, and others mentioned in the script).  
2. **AWS Credentials:** The script needs access to AWS credentials to connect to the SQS queue. These credentials can be provided via environment variables, AWS CLI configurations, or IAM roles.  
3. **Database Configuration:** The script relies on a database connection for processing account events. Ensure the `Database` class in the script is correctly configured to interact with your database.  
4. **Logging:** The script uses a custom logging utility (`util.configure_script_logging()`). Ensure the `util` module is available and properly configured.

#### Command-Line Arguments
- **`queue_name`** (Required): Name of the SQS queue to poll for events.
- **Options:**
  - `--aws-region`: Specify the AWS region of the queue (e.g., `us-west-2`). Defaults to the instance's AWS region.
  - `--queue-wait-time`: Number of seconds to wait for jobs on the queue (default: `20`).
  - `--verbose` (`-v`): Increase verbosity of log messages. Use multiple `-v` flags for higher verbosity levels.
  - `--human_logs`: Format logs for easier human readability.

### Usage

Run the script using the following command:

```bash
python process_account_events.py [options] queue_name
```

#### Example

To process events from an SQS queue named `account-events-queue` in the `us-west-2` region:

```bash
python process_account_events.py --aws-region us-west-2 account-events-queue
```