# Purge Expired TTLs
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

import argparse
import json
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


def use_dsn(args):
    try:
        if not args.sync_database_url:
            raise Exception("no url")
        url = args.sync_database_url
        purl = parse.urlparse(url)
        if purl.scheme == "spanner":
            path = purl.path.split("/")
            args.instance_id = path[-3]
            args.database_id = path[-1]
    except Exception as e:
        # Change these to reflect your Spanner instance install
        print("Exception {}".format(e))
    return args


def deleter(database, name, query):
    with statsd.timer("syncstorage.purge_ttl.{}_duration".format(name)):
        logging.info("Running: {}".format(query))
        start = datetime.now()
        result = database.execute_partitioned_dml(query)
        end = datetime.now()
        logging.info(
            "{name}: removed {result} rows, {name}_duration: {time}".format(
            name=name, result=result, time=end - start))


def add_conditions(args, query):
    if args.collection_ids:
        query += " AND collection_id"
        if len(args.collection_ids) == 1:
            query += " = {:d}".format(args.collection_ids[0])
        else:
            query += " in ({})".format(
                ', '.join(map(str, args.collection_ids)))
    return query


def spanner_purge(args, request=None):
    instance = client.instance(args.instance_id)
    database = instance.database(args.database_id)

    logging.info("For {}:{}".format(args.instance_id, args.database_id))
    batch_query = 'DELETE FROM batches WHERE expiry < CURRENT_TIMESTAMP()'
    bso_query = add_conditions(
            args,
            'DELETE FROM bsos WHERE expiry < CURRENT_TIMESTAMP()'
        )

    # Delete Batches. Also deletes child batch_bsos rows (INTERLEAVE
    # IN PARENT batches ON DELETE CASCADE)

    deleter(
        database,
        name="batches",
        query=batch_query
    )
    # Delete BSOs
    deleter(
        database,
        name="bso",
        query=bso_query
    )


def get_args():
    parser = argparse.ArgumentParser(
        description="Purge old TTLs"
    )
    parser.add_argument(
        "-i",
        "--instance_id",
        default=os.environ.get("INSTANCE_ID", "spanner-test"),
        help="Spanner instance ID"
    )
    parser.add_argument(
        "-d",
        "--database_id",
        default=os.environ.get("DATABASE_ID", "sync_schema3"),
        help="Spanner Database ID"
    )
    parser.add_argument(
        "-u",
        "--sync_database_url",
        default=os.environ.get("SYNC_DATABASE_URL"),
        help="Spanner Database DSN"
    )
    parser.add_argument(
        "--collection_ids",
        default=os.environ.get("COLLECTION_IDS", "[]"),
        help="JSON array of collection IDs to purge"
    )
    args = parser.parse_args()
    collections = json.loads(args.collection_ids)
    if not isinstance(collections, list):
        collections = [collections]
    args.collection_ids = collections
    # override using the DSN URL:
    if args.sync_database_url:
        args = use_dsn(args)
    return args


if __name__ == "__main__":
    args = get_args()
    with statsd.timer("syncstorage.purge_ttl.total_duration"):
        start_time = datetime.now()
        logging.info('Starting purge_ttl.py')

        spanner_purge(args)

        end_time = datetime.now()
        duration = end_time - start_time
        logging.info(
            'Completed purge_ttl.py, total_duration: {}'.format(duration))
