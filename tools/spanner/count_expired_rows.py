# Count the number of users in the spanner database
# Specifically, the number of unique fxa_uid found in the user_collections table
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

import sys
import logging
from statsd.defaults.env import statsd

from google.cloud import spanner
from tools.spanner.utils import ids_from_env

# set up logger
logging.basicConfig(
    format='{"datetime": "%(asctime)s", "message": "%(message)s"}',
    stream=sys.stdout,
    level=logging.INFO,
)

# Change these to match your install.
client = spanner.Client()


def spanner_read_data(query: str, table: str) -> None:
    """
    Executes a query on the specified Spanner table to count expired rows,
    logs the result, and sends metrics to statsd.

    Args:
        query (str): The SQL query to execute.
        table (str): The name of the table being queried.
    Returns:
        None
    """
    (instance_id, database_id, project_id) = ids_from_env()
    instance = client.instance(instance_id)
    database = instance.database(database_id)

    logging.info(f"For {instance_id}:{database_id} {project_id}")

    # Count expired rows in the specified table
    with statsd.timer(f"syncstorage.count_expired_{table}_rows.duration"):
        with database.snapshot() as snapshot:
            result = snapshot.execute_sql(query)
            row_count = result.one()[0]
            statsd.gauge(f"syncstorage.expired_{table}_rows", row_count)
            logging.info(f"Found {row_count} expired rows in {table}")


if __name__ == "__main__":
    logging.info("Starting count_expired_rows.py")

    for table in ["batches", "bsos"]:
        query = f"SELECT COUNT(*) FROM {table} WHERE expiry < CURRENT_TIMESTAMP()"
        spanner_read_data(query, table)

    logging.info("Completed count_expired_rows.py")
