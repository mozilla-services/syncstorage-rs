# Purge Expired TTLs
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

import argparse
import logging
import os
import sys
from datetime import datetime
from typing import List, Optional
from urllib import parse

from google.cloud import spanner
from google.cloud.spanner_v1.database import Database
from statsd.defaults.env import statsd

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


def deleter(database: Database, name: str, query: str, prefix: Optional[str]):
    with statsd.timer("syncstorage.purge_ttl.{}_duration".format(name)):
        logging.info("Running: {}".format(query))
        start = datetime.now()
        result = database.execute_partitioned_dml(query)
        end = datetime.now()
        logging.info(
            "{name}: removed {result} rows, {name}_duration: {time}, prefix: {prefix}".format(
                name=name, result=result, time=end - start, prefix=prefix))


def add_conditions(args, query: str, prefix: Optional[str]):
    """
    Add SQL conditions to a query.
    :param args: The program arguments
    :param query: The SQL query
    :param prefix: The current prefix, if given
    :return: The updated SQL query
    """
    if args.collection_ids:
        query += " AND collection_id"
        if len(args.collection_ids) == 1:
            query += " = {:d}".format(args.collection_ids[0])
        else:
            query += " in ({})".format(
                ', '.join(map(str, args.collection_ids)))
    if prefix:
        query += ' AND REGEXP_CONTAINS(fxa_uaid, r"{}")'.format(prefix)
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
    expiry_condition = get_expiry_condition(args)
    prefixes = args.uid_prefixes if args.uid_prefixes else [None]

    for prefix in prefixes:
        logging.info("For {}:{}, prefix = {}".format(args.instance_id, args.database_id, prefix))

        if args.mode in ["batches", "both"]:
            # Delete Batches. Also deletes child batch_bsos rows (INTERLEAVE
            # IN PARENT batches ON DELETE CASCADE)
            batch_query = 'DELETE FROM batches WHERE {}'.format(expiry_condition)
            deleter(
                database,
                name="batches",
                query=batch_query,
                prefix=prefix
            )

        if args.mode in ["bsos", "both"]:
            # Delete BSOs
            bso_query = add_conditions(
                args,
                'DELETE FROM bsos WHERE {}'.format(expiry_condition),
                prefix
            )
            deleter(
                database,
                name="bso",
                query=bso_query,
                prefix=prefix
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
        type=parse_args_list,
        default=os.environ.get("COLLECTION_IDS", "[]"),
        help="Array of collection IDs to purge"
    )
    parser.add_argument(
        "--uid_prefixes",
        type=parse_args_list,
        default=os.environ.get("PURGE_UID_PREFIXES", "[]"),
        help="Array of regex strings used to limit purges based on UID. "
             "Each entry is a separate purge run."
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

    # override using the DSN URL:
    if args.sync_database_url:
        args = use_dsn(args)

    return args


def parse_args_list(args_list: str) -> List[str]:
    """
    Parse a list of items (or a single string) into a list of strings.
    Example input: [item1,item2,item3]
    :param args_list: The list/string
    :return: A list of strings
    """
    if args_list[0] != "[" or args_list[-1] != "]":
        # Assume it's a single item
        return [args_list]

    return args_list[1:-1].split(",")


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
