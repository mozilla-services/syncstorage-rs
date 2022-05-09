# Preload Spanner Database
#
# Spanner increases efficiency when there is a minimum of 300G of
# data stored. This script preloads a minimal set of data to trigger
# that level of optimization.
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
import os
from urllib import parse

import random
import string
import uuid
from datetime import datetime, timedelta

import threading

from google.api_core.exceptions import AlreadyExists
from google.cloud import spanner
from google.cloud.spanner_v1 import param_types


# max batch size for this write is 2000, otherwise we run into:
"""google.api_core.exceptions.InvalidArgument: 400 The transaction
contains too many mutations. Insert and update operations count with
the multiplicity of the number of columns they affect. For example,
inserting values into one key column and four non-key columns count as
five mutations total for the insert. Delete and delete range
operations count as one mutation regardless of the number of columns
affected. The total mutation count includes any changes to indexes
that the transaction generates. Please reduce the number of writes, or
use fewer indexes. (Maximum number: 20000)

or

google.api_core.exceptions.ResourceExhausted: 429 Received message
larger than max (1000422248 vs. 104857600)

or

google.api_core.exceptions.InvalidArgument: 400 The transaction
exceeds the maximum total bytes-size that can be handled by
Spanner. Please reduce the size or number of the writes, or use fewer
indexes. (Maximum size: 104857600)

"""
# 1 Batch of 2K records with payload of 25K = 201_168_000B
# so, ~300G would need 2_982_582 batches
BATCH_SIZE = 2000
# Total number of threads to use
THREAD_COUNT = 16
# Number of batches per thread
BATCHES = 330

# `100` is the bottom limit for reserved collections.
COLL_ID = 100

# The following can trigge OOMs
# PAYLOAD_SIZE = 2500000
# PAYLOAD_SIZE = 1000000
"""
google.api_core.exceptions.InvalidArgument: 400 The transaction exceeds
the maximum total bytes-size that can be handled by Spanner. Please reduce the
size or number of the writes, or use fewer indexes. (Maximum size: 104857600)
"""
# PAYLOAD_SIZE = 50000
PAYLOAD_SIZE = 25000
# fake a base64 like payload. Not strictly neccessary, but may help ML
# routines.
PAYLOAD = ''.join(
    random.choice(
        string.digits + string.ascii_uppercase + string.ascii_lowercase + "-_="
    )
    for _ in range(PAYLOAD_SIZE))


def load(instance, db, coll_id, name):
    fxa_uid = "DEADBEEF" + uuid.uuid4().hex[8:]
    fxa_kid = "{:013d}-{}".format(22, fxa_uid)
    print("{} -> Loading {} {}".format(name, fxa_uid, fxa_kid))
    name = threading.current_thread().getName()
    spanner_client = spanner.Client()
    instance = spanner_client.instance(instance)
    db = instance.database(db)
    print('{name} Db: {db}'.format(name=name, db=db))
    start = datetime.now()

    def create_user(txn):
        txn.execute_update(
            """\
            INSERT INTO user_collections
                (fxa_uid, fxa_kid, collection_id, modified)
            VALUES (@fxa_uid, @fxa_kid, @collection_id, @modified)
            """,
            params=dict(
                fxa_uid=fxa_uid,
                fxa_kid=fxa_kid,
                collection_id=coll_id,
                modified=start
            ),
            param_types=dict(
                fxa_uid=param_types.STRING,
                fxa_kid=param_types.STRING,
                collection_id=param_types.INT64,
                modified=param_types.TIMESTAMP
            )
        )

    try:
        db.run_in_transaction(create_user)
        print('{name} Created user (fxa_uid: {uid}, fxa_kid: {kid})'.format(
            name=name, uid=fxa_uid, kid=fxa_kid))
    except AlreadyExists:
        print('{name} Existing user (fxa_uid: {uid}}, fxa_kid: {kid}})'.format(
              name=name, uid=fxa_uid, kid=fxa_kid))

    # approximately 1892 bytes
    rlen = 0

    print('{name} Loading..'.format(name=name))
    for j in range(BATCHES):
        records = []
        for i in range(BATCH_SIZE):
            # create a record
            record = (
                fxa_uid,
                fxa_kid,
                coll_id,
                str(uuid.uuid4()),
                None,
                PAYLOAD,
                start,
                start + timedelta(days=365 * 5)
            )
            # determine it's size.
            rlen = len(record[1]) * 4
            rlen += 64
            rlen += len(record[3]) * 4
            rlen += 64
            rlen += len(record[5]) * 4
            rlen += 64
            rlen += 64
            records.append(record)
        with db.batch() as batch:
            batch.insert(
                table='bsos',
                columns=(
                    'fxa_uid',
                    'fxa_kid',
                    'collection_id',
                    'bso_id',
                    'sortindex',
                    'payload',
                    'modified',
                    'expiry'
                ),
                values=records
            )
        print(
            ('{name} Wrote batch {b} of {bb}:'
             ' {c} records {r} bytes, {t}').format(
                name=name,
                b=j + 1,
                bb=BATCHES,
                c=BATCH_SIZE,
                r=rlen,
                t=datetime.now() - start))
    print('{name} Total: {t} (count: {c}, size: {s} in {sec})'.format(
        name=name,
        t=BATCHES,
        c=BATCHES * BATCH_SIZE,
        s=BATCHES * BATCH_SIZE * rlen,
        sec=datetime.now() - start
    ))


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


def loader():
    # Prefix uaids for easy filtering later
    # Each loader thread gets it's own fake user to prevent some hotspot
    # issues.
    (instance_id, database_id) = from_env()
    # switching uid/kid to per load because of weird google trimming
    name = threading.current_thread().getName()
    load(instance_id, database_id, COLL_ID, name)


def main():
    for c in range(THREAD_COUNT):
        print("Starting thread {}".format(c))
        t = threading.Thread(
            name="loader_{}".format(c),
            target=loader)
        t.start()


if __name__ == '__main__':
    main()
