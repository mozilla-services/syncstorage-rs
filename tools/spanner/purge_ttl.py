# Purge Expired TTLs
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

import os
from google.cloud import spanner

# Change these to match your install. 
instance_id = os.environ.get("INSTANCE_ID", 'spanner-test')
database_id = os.environ.get("DATABASE_ID", 'sync_stage')

client = spanner.Client()


def spanner_read_data(request=None):
    instance = client.instance(instance_id)
    database = instance.database(database_id)
    outputs = []

    outputs.append("For {}:{}".format(instance_id, database_id))
    # Delete Batches
    query = 'DELETE FROM batches WHERE expiry < CURRENT_TIMESTAMP()'
    result = database.execute_partitioned_dml(query)
    outputs.append("batches: removed {} rows".format(result))

    # Delete BSOs
    query = 'DELETE FROM bso WHERE expiry < CURRENT_TIMESTAMP()'
    result = database.execute_partitioned_dml(query)
    outputs.append("bso: removed {} rows".format(result))
    return '\n'.join(outputs)

if __name__ == "__main__":
    print(spanner_read_data())