# Purge Expired TTLs
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

import os
from urllib import parse

from google.cloud import spanner

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
    outputs = []

    outputs.append("For {}:{}".format(instance_id, database_id))
    # Delete Batches. Also deletes child batch_bsos rows (INTERLEAVE
    # IN PARENT batches ON DELETE CASCADE)
    query = 'DELETE FROM batches WHERE expiry < CURRENT_TIMESTAMP()'
    result = database.execute_partitioned_dml(query)
    outputs.append("batches: removed {} rows".format(result))

    # Delete BSOs
    query = 'DELETE FROM bsos WHERE expiry < CURRENT_TIMESTAMP()'
    result = database.execute_partitioned_dml(query)
    outputs.append("bso: removed {} rows".format(result))
    return '\n'.join(outputs)


if __name__ == "__main__":
    print(spanner_read_data())
