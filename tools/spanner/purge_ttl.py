# Purge Expired TTLs
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
logger = logging.getLogger()
logger.setLevel(logging.INFO)
handler = logging.StreamHandler(sys.stdout)
formatter = logging.Formatter('{"datetime": "%(asctime)s", "message": "%(message)s"}')
handler.setFormatter(formatter)
logger.addHandler(handler)

# Change these to match your install.
client = spanner.Client()


def from_env():
    try:
        url = os.environ.get("SYNC_DATABASE_URL")
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

    logger.info("For {}:{}".format(instance_id, database_id))

    # Delete Batches. Also deletes child batch_bsos rows (INTERLEAVE
    # IN PARENT batches ON DELETE CASCADE)
    batches_start = datetime.now()
    query = 'DELETE FROM batches WHERE expiry < CURRENT_TIMESTAMP()'
    result = database.execute_partitioned_dml(query)
    batches_end = datetime.now()
    batches_duration = batches_end - batches_start
    logger.info("batches: removed {} rows, batches_duration: {}".format(result, batches_duration))
    statsd.timing("sync.purge_ttl.batches_duration", batches_duration)

    # Delete BSOs
    bso_start = datetime.now()
    query = 'DELETE FROM bsos WHERE expiry < CURRENT_TIMESTAMP()'
    result = database.execute_partitioned_dml(query)
    bso_end = datetime.now()
    bso_duration = bso_end - bso_start
    logger.info("bso: removed {} rows, bso_duration: {}".format(result, bso_duration))
    statsd.timing("sync.purge_ttl.bso_duration", bso_duration)


if __name__ == "__main__":
    start_time = datetime.now()
    logger.info('Starting purge_ttl.py')

    spanner_read_data()

    end_time = datetime.now()
    duration = end_time - start_time
    logger.info('Completed purge_ttl.py, total_duration: {}'.format(duration))
    statsd.timing("sync.purge_ttl.total_duration", duration)
