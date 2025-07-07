# Count the number of users in the spanner database
# Specifically, the number of unique fxa_uid found in the user_collections table
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

import os
import sys
import logging
from datetime import datetime
from statsd.defaults.env import statsd
from urllib import parse

from google.cloud import spanner

# set up logger
logging.basicConfig(
    format='{"datetime": "%(asctime)s", "message": "%(message)s"}',
    stream=sys.stdout,
    level=logging.INFO)

# Change these to match your install.
client = spanner.Client()


def from_env():
    """
    Function that extracts the instance, project, and database ids from the DSN url.
    It is defined as the SYNC_SYNCSTORAGE__DATABASE_URL environment variable.
    The defined defaults are in webservices-infra/sync and can be configured there for
    production runs. 

    For reference, an example spanner url passed in is in the following format:
    `spanner://projects/moz-fx-sync-prod-xxxx/instances/sync/databases/syncdb`
    database_id = `syncdb`, instance_id = `sync`, project_id = `moz-fx-sync-prod-xxxx`
    """
    try:
        url = os.environ.get("SYNC_SYNCSTORAGE__DATABASE_URL")
        if not url:
            raise Exception("no url")
        parsed_url = parse.urlparse(url)
        if parsed_url.scheme == "spanner":
            path = parsed_url.path.split("/")
            instance_id = path[-3]
            project_id = path[-5]
            database_id = path[-1]
    except Exception as e:
        # Change these to reflect your Spanner instance install
        print(f"Exception {e}")
        instance_id = os.environ.get("INSTANCE_ID", "spanner-test")
        database_id = os.environ.get("DATABASE_ID", "sync_stage")
        project_id = os.environ.get("GOOGLE_CLOUD_PROJECT", "test-project")
    return (instance_id, database_id, project_id)


def spanner_read_data(request=None):
    (instance_id, database_id, project_id) = from_env()
    instance = client.instance(instance_id)
    database = instance.database(database_id)
    project = instance.database(database_id)

    logging.info(f"For {instance_id}:{database_id} {project}")

    # Count users
    with statsd.timer("syncstorage.count_users.duration"):
        with database.snapshot() as snapshot:
            query = 'SELECT COUNT (DISTINCT fxa_uid) FROM user_collections'
            result = snapshot.execute_sql(query)
            user_count = result.one()[0]
            statsd.gauge("syncstorage.distinct_fxa_uid", user_count)
            logging.info(f"Count found {user_count} distinct users")


if __name__ == "__main__":
    logging.info('Starting count_users.py')

    spanner_read_data()

    logging.info('Completed count_users.py')
