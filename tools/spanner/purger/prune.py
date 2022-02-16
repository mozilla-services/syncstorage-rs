# Purge old KIDs from UIDs

import argparse
import logging
import os
import sys
import re
from datetime import datetime
from typing import List
from urllib import parse

from google.cloud import spanner
from google.cloud.spanner_v1 import param_types
from statsd.defaults.env import statsd


logging.basicConfig(
    format='{"datetime": "%(asctime)s", "message": "%(message)s"}',
    stream=sys.stdout,
    level=logging.DEBUG)

client = spanner.Client()


def kill_abandoned(database, prefix, age, args):
    """kill all records that have not been touched in $age"""
    find_sql = (
        "select fxa_uid, fxa_kid from bsos where collection_id=6 and "
        "starts_with(fxa_uid, @prefix) and "
        "timestamp_diff(CURRENT_TIMESTAMP(), modified, day) > @age")
    del_sql = "DELETE FROM bsos WHERE fxa_uid=@uid AND fxa_kid=@kid"
    params = {"prefix": prefix, "age": age}
    types = {"prefix": param_types.STRING, "age": param_types.INT64}
    with database.snapshot() as snap:
        results = snap.execute_sql(
            find_sql,
            params=params,
            param_types=types
        )
        retirees = list(results)
        if not retirees:
            logging.info("No retirees found")
            return
        logging.info("Found {} retirees".format(len(retirees)))
        for retiree in retirees:
            (uid, kid) = retiree
            logging.info("deleting {}:{}".format(uid, kid))
            if args.dryrun:
                logging.debug(
                    "DELETE FROM bsos WHERE "
                    "fxa_uid={uid} and fxa_kid={kid}".format(
                        uid=uid,
                        kid=kid
                    )
                )
            else:
                database.execute_partitioned_dml(
                    del_sql,
                    params={"uid": uid, "kid": kid},
                    param_types={
                        "uid": param_types.STRING,
                        "kid": param_types.STRING}
                )
    return len(retirees)


def find_kids(database, prefix, args):
    """find all uids with multiple kids"""
    sql = (
        "select fxa_uid from bsos where collection_id=6 and "
        "starts_with(fxa_uid, @prefix) "
        "group by fxa_uid having count(fxa_kid) > 1")
    params = {"prefix": prefix}
    types = {"prefix": param_types.STRING}

    with database.snapshot() as snap:
        results = snap.execute_sql(
            sql,
            params=params,
            param_types=types
        )
        return [item[0] for item in results]


def kill_kids(database, uid, args):
    """kill all the no longer accessable kids for a given UID"""
    sql = """delete
             from bsos
             where fxa_uid = @uid
                and fxa_kid in (
                    select fxa_kid
                    from bsos
                    where collection_id = 6
                        and fxa_uid = @uid
                    order by modified desc
                    limit 100
                    offset 1)"""
    params = {"uid": uid}
    types = {"uid": param_types.STRING}
    result = 0

    start = datetime.now()
    if not args.dryrun:
        result = database.execute_partitioned_dml(
            sql, params=params, param_types=types)
    else:
        logging.debug(re.sub(r"\s+", " ", sql.replace("@uid", uid)))
    end = datetime.now()
    logging.info(
        "removed {result} rows, duration: {time}, uid: {uid}".format(
            result=result, time=end - start, uid=uid
        )
    )


def spanner_prune(args):
    """master pruning function for syncstorage"""
    # get the list of uaids with multiple kids
    database = client.instance(args.instance_id).database(args.database_id)
    if args.auto_split:
        args.uid_prefixes = [
            hex(i).lstrip("0x").zfill(args.auto_split) for i in range(
                0, 16 ** args.auto_split)]
    prefixes = args.uid_prefixes if args.uid_prefixes else [None]

    for prefix in prefixes:
        logging.info("For {}:{} prefix = {}".format(
            args.instance_id,
            args.database_id,
            prefix))

        if args.abandon_age:
            if args.abandon_age < 300:
                logging.error("Too young to kill. Try again.")
                return()
            kill_abandoned(database, prefix, args.abandon_age, args)

        parents = find_kids(database, prefix, args)
        if len(parents) == 0:
            logging.debug("No parents")
            continue

        logging.info("Found {c} candidates".format(
            c=len(parents)
        ))
        for uid in parents:
            kill_kids(database, uid, args)


def use_dsn(args):
    """parse a spanner DSN"""
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


def get_args():
    """Parse all the arguments"""
    parser = argparse.ArgumentParser(
        description="Prune old KIDs"
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
        "--uid_prefixes",
        "--prefix",
        type=parse_args_list,
        default=os.environ.get("PURGE_UID_PREFIXES", "[]"),
        help="Array of strings used to limit purges based on UID. "
             "Each entry is a separate purge run."
    )
    parser.add_argument(
        "--auto_split",
        type=int,
        default=os.environ.get("PURGE_AUTO_SPLIT"),
        help="""Automatically generate `uid_prefixes` for this many digits, """
             """(e.g. `3` would produce """
             """`uid_prefixes=["000","001","002",...,"fff"])"""
    )
    parser.add_argument(
        "--abandon_age",
        default=os.environ.get("SYNC_ABANDON_AGE", 366),
        help="remove all records that have not been modified in"
             "this many days"
    )
    parser.add_argument(
        '--dryrun',
        action="store_true",
        help="Do not purge user records from spanner"
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

        spanner_prune(args)

        end_time = datetime.now()
        duration = end_time - start_time
        logging.info(
            'Completed purge_ttl.py, total_duration: {}'.format(duration))
