# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

import argparse
import logging
import os
import sys
from datetime import datetime
from typing import List, Optional, Tuple
from urllib.parse import urlparse

import sqlalchemy
from sqlalchemy import text
from statsd.defaults.env import statsd

logging.basicConfig(
    format='{"datetime": "%(asctime)s", "message": "%(message)s"}',
    stream=sys.stdout,
    level=logging.INFO,
)


def get_db_engine(database_url: str):
    """Create a SQLAlchemy engine from a database url."""
    parsed = urlparse(database_url)
    if parsed.scheme not in ['postgresql', 'postgres']:
        raise ValueError(f"Unsupported database scheme: {parsed.scheme}")

    # newer versions of SQLAlchemy want 'postgresql' instead of 'postgres'
    if parsed.scheme == 'postgres':
        parsed = parsed._replace(scheme='postgresql')

    return sqlalchemy.create_engine(parsed.geturl())


def exec_delete(
    engine,
    name: str,
    query: str,
    params: Optional[dict] = None,
    dryrun: Optional[bool] = False,
):
    """Execute the DELETE query with the given query params."""
    with statsd.timer(f"syncstorage.purge_ttl.{name}_duration"):
        logging.info(f"Running: {query} :: {params}")
        start = datetime.now()
        result = 0

        if not dryrun:
            with engine.connect() as conn:
                result_proxy = conn.execute(text(query), params or {})
                result = result_proxy.rowcount
        end = datetime.now()
        logging.info(
            f"{name}: removed {result} rows, {name}_duration: {end - start}"
        )


def add_conditions(args, query: str) -> Tuple[str, dict]:
    """Add SQL conditions to the query and store the arg values."""
    params = {}

    if args.collection_ids:
        ids = list(filter(len, args.collection_ids))
        if ids:
            query += " AND collection_id"
            if len(ids) == 1:
                query += " = :collection_id"
                params["collection_id"] = ids[0]
            else:
                param_names = [f"collection_id_{i}" for i in range(len(ids))]
                query += f" IN ({', '.join([':' + name for name in param_names])})"
                for i, cid in enumerate(ids):
                    params[f"collection_id_{i}"] = cid

    return (query, params)


def get_expiry_condition(args) -> str:
    """Build the expiry WHERE condition."""
    if args.expiry_mode == "now":
        return "expiry < CURRENT_TIMESTAMP"
    elif args.expiry_mode == "midnight":
        return "expiry < DATE_TRUNC('day', CURRENT_TIMESTAMP AT TIME ZONE 'UTC')"
    else:
        raise Exception(f"Invalid expiry mode: {args.expiry_mode}")


def purge_records(args) -> None:
    """The main fn."""
    engine = get_db_engine(args.database_url)
    expiry_condition = get_expiry_condition(args)

    if args.mode in ["batches", "both"]:
        (batch_query, params) = add_conditions(
            args,
            f"DELETE FROM batches WHERE {expiry_condition}",
        )
        exec_delete(
            engine,
            name="batches",
            query=batch_query,
            params=params,
            dryrun=args.dryrun,
        )

    if args.mode in ["bsos", "both"]:
        (bso_query, params) = add_conditions(
            args,
            f"DELETE FROM bsos WHERE {expiry_condition}",
        )
        exec_delete(
            engine,
            name="bsos",
            query=bso_query,
            params=params,
            dryrun=args.dryrun,
        )


def get_args():
    """Parse cli args."""
    parser = argparse.ArgumentParser(description="Purge expired records from the database")
    parser.add_argument(
        "-u",
        "--database_url",
        default=os.environ.get("SYNC_SYNCSTORAGE__DATABASE_URL"),
        required=False,
        help="Database URL (postgresql://... or postgres://...)",
    )
    parser.add_argument(
        "--collection_ids",
        "--ids",
        type=parse_args_list,
        default=os.environ.get("COLLECTION_IDS", "[]"),
        help="Array of collection IDs to purge",
    )
    parser.add_argument(
        "--mode",
        type=str,
        choices=["batches", "bsos", "both"],
        default=os.environ.get("PURGE_MODE", "both"),
        help="Purge batches, bsos, or both",
    )
    parser.add_argument(
        "--expiry_mode",
        type=str,
        choices=["now", "midnight"],
        default=os.environ.get("PURGE_EXPIRY_MODE", "midnight"),
        help="Choose the timestamp used to check if an entry is expired",
    )
    parser.add_argument(
        "--dryrun",
        action="store_true",
        help="A dry run instead of purging data",
    )

    args = parser.parse_args()

    if not args.database_url:
        parser.error("--database_url is required, or set SYNC_SYNCSTORAGE__DATABASE_URL")

    return args


def parse_args_list(args_list: str) -> List[str]:
    """Parses a string representing a list of items into a list of strings.

    Args:
        args_list (str): String to parse, e.g., "[item1,item2,item3]" or "item1".

    Returns:
        List[str]: List of parsed string items.
    """
    if not args_list or args_list == "[]":
        return []

    if args_list[0] != "[" or args_list[-1] != "]":
        # Assume it's a single item
        return [args_list]

    return args_list[1:-1].split(",")


if __name__ == "__main__":
    args = get_args()
    with statsd.timer("syncstorage.purge_ttl.total_duration"):
        start_time = datetime.now()
        logging.info("Starting purge_ttl.py")

        purge_records(args)

        end_time = datetime.now()
        duration = end_time - start_time
        logging.info(f"Completed purge_ttl.py, total_duration: {duration}")
