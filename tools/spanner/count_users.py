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
from utils import ids_from_env

# set up logger
logging.basicConfig(
    format='{"datetime": "%(asctime)s", "message": "%(message)s"}',
    stream=sys.stdout,
    level=logging.INFO,
)

# Change these to match your install.
client = spanner.Client()


def spanner_read_data() -> None:
    """
    Reads data from a Google Cloud Spanner database to count the number of distinct users.

    This function connects to a Spanner instance and database using environment variables,
    executes a SQL query to count the number of distinct `fxa_uid` entries in the `user_collections` table,
    and logs the result. It also records the duration of the operation and the user count using statsd metrics.

    Args:
        None

    Returns:
        None
    """
    (instance_id, database_id, project_id) = ids_from_env()
    instance = client.instance(instance_id)
    database = instance.database(database_id)
    project = instance.database(database_id)

    logging.info(f"For {instance_id}:{database_id} {project}")

    # Count users
    with statsd.timer("syncstorage.count_users.duration"):
        with database.snapshot() as snapshot:
            query = "SELECT COUNT (DISTINCT fxa_uid) FROM user_collections"
            result = snapshot.execute_sql(query)
            user_count = result.one()[0]
            statsd.gauge("syncstorage.distinct_fxa_uid", user_count)
            logging.info(f"Count found {user_count} distinct users")


if __name__ == "__main__":
    logging.info("Starting count_users.py")

    spanner_read_data()

    logging.info("Completed count_users.py")
