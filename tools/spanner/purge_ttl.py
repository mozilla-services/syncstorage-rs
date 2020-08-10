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
    if args.uid_starts:
        query += " AND fxa_uid LIKE \"{}%\"".format(args.uid_starts)
    return query


def get_expiry_condition(args):
    """
    Get the expiry SQL WHERE condition to use
    :param args: The program arguments
    :return: A SQL snippet to use in the WHERE clause
    """
    if args.expiry_mode == "now":
        return 'expiry < CURRENT_TIMESTAMP()'
    elif args.expiry_mode == "midnight":
        return 'expiry < TIMESTAMP_TRUNC(CURRENT_TIMESTAMP(), DAY, "UTC")'
    else:
        raise Exception("Invalid expiry mode: {}".format(args.expiry_mode))


def spanner_purge(args):
    instance = client.instance(args.instance_id)
    database = instance.database(args.database_id)

    logging.info("For {}:{}".format(args.instance_id, args.database_id))
    expiry_condition = get_expiry_condition(args)
    batch_query = (
        'DELETE FROM batches WHERE {}'.format(expiry_condition)
    )
    bso_query = add_conditions(
        args,
        'DELETE FROM bsos WHERE {}'.format(expiry_condition)
    )

    if args.mode in ["batches", "both"]:
        # Delete Batches. Also deletes child batch_bsos rows (INTERLEAVE
        # IN PARENT batches ON DELETE CASCADE)
        deleter(
            database,
            name="batches",
            query=batch_query
        )

    if args.mode in ["bsos", "both"]:
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
    parser.add_argument(
        "--uid_starts",
        type=str,
        help="Limit to UIDs starting with specified characters"
    )
    parser.add_argument(
        "--mode",
        type=str,
        choices=["batches", "bsos", "both"],
        default=os.environ.get("PURGE_MODE", "both"),
        help="Purge TTLs in batches, bsos, or both"
    )
    parser.add_argument(
        "--expiry_mode",
        type=str,
        choices=["now", "midnight"],
        default=os.environ.get("PURGE_EXPIRY_MODE", "midnight"),
        help="Choose the timestamp used to check if an entry is expired"
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
