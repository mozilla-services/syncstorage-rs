# Purge Expired TTLs
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

from google.cloud import spanner

# Change these to match your install. 
instance_id = 'spanner-test'
database_id = 'sync_kid'

client = spanner.Client()


def spanner_read_data(request):
    instance = client.instance(instance_id)
    database = instance.database(database_id)

    query = 'DELETE FROM bso WHERE ttl < TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 30 DAY)'

    result = database.execute_partitioned_dml(query)
    outputs.append(results)
    return '\n'.join(outputs)
