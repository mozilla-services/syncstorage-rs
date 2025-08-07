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


from google.cloud import spanner
from google.cloud.spanner_v1.database import Database
from google.cloud.spanner_v1 import param_types
from statsd.defaults.env import statsd

from tools.spanner.utils import ids_from_env, Mode

# set up logger
logging.basicConfig(
    format='{"datetime": "%(asctime)s", "message": "%(message)s"}',
    stream=sys.stdout,
    level=logging.INFO,
)

# Change these to match your install.
client = spanner.Client()


def deleter(
    database: Database,
    name: str,
    query: str,
    prefix: Optional[str] = None,
    params: Optional[dict] = None,
    param_types: Optional[dict] = None,
    dryrun: Optional[bool] = False,
):
    with statsd.timer(f"syncstorage.purge_ttl.{name}_duration"):
        logging.info(f"Running: {query} :: {params}")
        start = datetime.now()
        result = 0
        if not dryrun:
            result = database.execute_partitioned_dml(
                query, params=params, param_types=param_types
            )
        end = datetime.now()
        logging.info(
            f"{name}: removed {result} rows, {name}_duration: {end - start}, prefix: {prefix}"
        )


def add_conditions(args, query: str, prefix: Optional[str]):
    """
    Add SQL conditions to a query.
    :param args: The program arguments
    :param query: The SQL query
    :param prefix: The current prefix, if given
    :return: The updated SQL query, and list of params
    """
    params = {}
    types = {}
    if args.collection_ids:
        ids = list(filter(len, args.collection_ids))
        if ids:
            query += " AND collection_id"
            if len(ids) == 1:
                query += " = @collection_id"
                params["collection_id"] = ids[0]
                types["collection_id"] = param_types.INT64
            else:
                for count, id in enumerate(ids):
                    name = f"collection_id_{count}"
                    params[name] = id
                    types[name] = param_types.INT64
                query += " in (@{})".format(", @".join(params.keys()))
    if prefix:
        query += " AND STARTS_WITH(fxa_uid, @prefix)"
        params["prefix"] = prefix
        types["prefix"] = param_types.STRING
    return (query, params, types)


def get_expiry_condition(args):
    """
    Get the expiry SQL WHERE condition to use
    :param args: The program arguments
    :return: A SQL snippet to use in the WHERE clause
    """
    if args.expiry_mode == "now":
        return "expiry < CURRENT_TIMESTAMP()"
    elif args.expiry_mode == "midnight":
        return 'expiry < TIMESTAMP_TRUNC(CURRENT_TIMESTAMP(), DAY, "UTC")'
    else:
        raise Exception(f"Invalid expiry mode: {args.expiry_mode}")


def spanner_purge(args) -> None:
    """
    Purges expired TTL records from Spanner based on the provided arguments.

    This function connects to the specified Spanner instance and database,
    determines the expiry condition, and deletes expired records from the
    'batches' and/or 'bsos' tables according to the purge mode. Supports
    filtering by collection IDs and UID prefixes, and can operate in dry-run mode.

    Args:
        args (argparse.Namespace): Parsed command-line arguments containing
            Spanner connection info, purge options, and filters.

    Returns:
        None
    """
    instance = client.instance(args.instance_id)
    database = instance.database(args.database_id)
    expiry_condition = get_expiry_condition(args)
    if args.auto_split:
        args.uid_prefixes = [
            hex(i).lstrip("0x").zfill(args.auto_split)
            for i in range(0, 16**args.auto_split)
        ]
    prefixes = args.uid_prefixes if args.uid_prefixes else [None]

    for prefix in prefixes:
        logging.info(f"For {args.instance_id}:{args.database_id}, prefix = {prefix}")

        if args.mode in ["batches", "both"]:
            # Delete Batches. Also deletes child batch_bsos rows (INTERLEAVE
            # IN PARENT batches ON DELETE CASCADE)
            (batch_query, params, types) = add_conditions(
                args,
                f"DELETE FROM batches WHERE {expiry_condition}",
                prefix,
            )
            deleter(
                database,
                name="batches",
                query=batch_query,
                params=params,
                param_types=types,
                prefix=prefix,
                dryrun=args.dryrun,
            )

        if args.mode in ["bsos", "both"]:
            # Delete BSOs
            (bso_query, params, types) = add_conditions(
                args, f"DELETE FROM bsos WHERE {expiry_condition}", prefix
            )
            deleter(
                database,
                name="bso",
                query=bso_query,
                params=params,
                param_types=types,
                prefix=prefix,
                dryrun=args.dryrun,
            )


def get_args():
    """
    Parses and returns command-line arguments for the Spanner TTL purge tool.
    If a DSN URL is provided, usually `SYNC_SYNCSTORAGE__DATABASE_URL`, its values override the corresponding arguments.

    Returns:
        argparse.Namespace: Parsed command-line arguments with the following attributes:
            - instance_id (str): Spanner instance ID (default from INSTANCE_ID env or 'spanner-test').
            - database_id (str): Spanner database ID (default from DATABASE_ID env or 'sync_schema3').
            - project_id (str): Google Cloud project ID (default from GOOGLE_CLOUD_PROJECT env or 'spanner-test').
            - sync_database_url (str): Spanner database DSN (default from SYNC_SYNCSTORAGE__DATABASE_URL env).
            - collection_ids (list): List of collection IDs to purge (default from COLLECTION_IDS env or empty list).
            - uid_prefixes (list): List of UID prefixes to limit purges (default from PURGE_UID_PREFIXES env or empty list).
            - auto_split (int): Number of digits to auto-generate UID prefixes (default from PURGE_AUTO_SPLIT env).
            - mode (str): Purge mode, one of 'batches', 'bsos', or 'both' (default from PURGE_MODE env or 'both').
            - expiry_mode (str): Expiry mode, either 'now' or 'midnight' (default from PURGE_EXPIRY_MODE env or 'midnight').
            - dryrun (bool): If True, do not actually purge records from Spanner.
    """
    parser = argparse.ArgumentParser(description="Purge old TTLs")
    parser.add_argument(
        "-i",
        "--instance_id",
        default=os.environ.get("INSTANCE_ID", "spanner-test"),
        help="Spanner instance ID",
    )
    parser.add_argument(
        "-d",
        "--database_id",
        default=os.environ.get("DATABASE_ID", "sync_schema3"),
        help="Spanner Database ID",
    )
    parser.add_argument(
        "-p",
        "--project_id",
        default=os.environ.get("GOOGLE_CLOUD_PROJECT", "spanner-test"),
        help="Spanner Project ID",
    )
    parser.add_argument(
        "-u",
        "--sync_database_url",
        default=os.environ.get("SYNC_SYNCSTORAGE__DATABASE_URL"),
        help="Spanner Database DSN",
    )
    parser.add_argument(
        "--collection_ids",
        "--ids",
        type=parse_args_list,
        default=os.environ.get("COLLECTION_IDS", "[]"),
        help="Array of collection IDs to purge",
    )
    parser.add_argument(
        "--uid_prefixes",
        "--prefix",
        type=parse_args_list,
        default=os.environ.get("PURGE_UID_PREFIXES", "[]"),
        help="Array of strings used to limit purges based on UID. "
        "Each entry is a separate purge run.",
    )
    parser.add_argument(
        "--auto_split",
        type=int,
        default=os.environ.get("PURGE_AUTO_SPLIT"),
        help="""Automatically generate `uid_prefixes` for this many digits, """
        """(e.g. `3` would produce """
        """`uid_prefixes=["000","001","002",...,"fff"])""",
    )
    parser.add_argument(
        "--mode",
        type=str,
        choices=["batches", "bsos", "both"],
        default=os.environ.get("PURGE_MODE", "both"),
        help="Purge TTLs in batches, bsos, or both",
    )
    parser.add_argument(
        "--expiry_mode",
        type=str,
        choices=["now", "midnight"],
        default=os.environ.get("PURGE_EXPIRY_MODE", "midnight"),
        help="Choose the timestamp used to check if an entry is expired",
    )
    parser.add_argument(
        "--dryrun", action="store_true", help="Do not purge user records from spanner"
    )
    args = parser.parse_args()

    # override using the DSN URL:
    if args.sync_database_url:
        (instance_id, database_id, project_id) = ids_from_env(
            args.sync_database_url, mode=Mode.URL
        )
        args.instance_id = instance_id
        args.database_id = database_id
        args.project_id = project_id
    return args


def parse_args_list(args_list: str) -> List[str]:
    """
    Parses a string representing a list of items into a list of strings.

    Args:
        args_list (str): String to parse, e.g., "[item1,item2,item3]" or "item1".

    Returns:
        List[str]: List of parsed string items.
    """
    if args_list[0] != "[" or args_list[-1] != "]":
        # Assume it's a single item
        return [args_list]

    return args_list[1:-1].split(",")


if __name__ == "__main__":
    args = get_args()
    with statsd.timer("syncstorage.purge_ttl.total_duration"):
        start_time = datetime.now()
        logging.info("Starting purge_ttl.py")

        spanner_purge(args)

        end_time = datetime.now()
        duration = end_time - start_time
        logging.info(f"Completed purge_ttl.py, total_duration: {duration}")
