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
    try:
        url = os.environ.get("SYNC_SYNCSTORAGE__DATABASE_URL")
        if not url:
            raise Exception("no url")
        purl = parse.urlparse(url)
        if purl.scheme == "spanner":
            path = purl.path.split("/")
            instance_id = path[-3]
            database_id = path[-1]
    except Exception as e:
        # Change these to reflect your Spanner instance install
        print("Exception {}".format(e))
        instance_id = os.environ.get("INSTANCE_ID", "spanner-test")
        database_id = os.environ.get("DATABASE_ID", "sync_stage")
    return (instance_id, database_id)


def spanner_read_data(request=None):
    (instance_id, database_id) = from_env()
    instance = client.instance(instance_id)
    database = instance.database(database_id)

    logging.info("For {}:{}".format(instance_id, database_id))

    # Count users
    with statsd.timer("syncstorage.count_users.duration"):
        with database.snapshot() as snapshot:
            query = 'SELECT COUNT (DISTINCT fxa_uid) FROM user_collections'
            result = snapshot.execute_sql(query)
            user_count = result.one()[0]
            statsd.gauge("syncstorage.distinct_fxa_uid", user_count)
            logging.info("Count found {} distinct users".format(user_count))


if __name__ == "__main__":
    logging.info('Starting count_users.py')

    spanner_read_data()

    logging.info('Completed count_users.py')
