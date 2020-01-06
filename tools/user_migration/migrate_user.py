#! venv/bin/python

# painfully stupid script to check out dumping mysql databases to avro.
# Avro is basically "JSON" for databases. It's not super complicated & it has
# issues (one of which is that it requires Python2).
#
#

import argparse
import base64
import time
from datetime import datetime

from mysql import connector
from google.cloud import spanner
from urllib.parse import urlparse

SPANNER_NODE_ID = 800


class BadDSNException(Exception):
    pass


def get_args():
    parser = argparse.ArgumentParser(
        description="move user from sql to spanner")
    parser.add_argument(
        '--dsns', default="move_dsns.lst",
        help="file of new line separated DSNs")
    parser.add_argument(
        '--users', default="move_users.lst",
        help="file of new line separated users to move")
    parser.add_argument(
        '--token_dsn',
        help="DSN to the token server database (optional)"
    )
    parser.add_argument(
        '--verbose',
        action="store_true",
        help="verbose logging"
    )
    return parser.parse_args()


def conf_mysql(dsn):
    """create a connection to the original storage system """
    connection = connector.connect(
        user=dsn.username,
        password=dsn.password,
        host=dsn.hostname,
        port=dsn.port or 3306,
        database=dsn.path[1:]
    )
    return connection


def conf_spanner(dsn):
    """create a connection to the new Spanner system"""
    path = dsn.path.split("/")
    instance_id = path[-3]
    database_id = path[-1]
    client = spanner.Client()
    instance = client.instance(instance_id)
    database = instance.database(database_id)
    return database


def conf_db(dsn):
    """read the list of storage definitions from the file and create
    a set of connetions.

     """
    if dsn.scheme == "mysql":
        return conf_mysql(dsn)
    if dsn.scheme == "spanner":
        return conf_spanner(dsn)
    raise RuntimeError("Unknown DNS type: {}".format(dsn.scheme))


def update_token(databases, user):
    """optionally update the TokenServer storage indicating the user
    is now on Spanner

    """
    cursor = databases['token'].cursor()
    cursor.execute(
        """
        UPDATE
            users
        SET
            replaced_at = {timestamp},
            nodeid = {nodeid}
        WHERE
           uid = {uid}
        """.format(
            timestamp=int(time.time() * 100),
            nodeid=SPANNER_NODE_ID,
            uid=user)
        )


# The following two functions are taken from browserid.utils
def encode_bytes_b64(value):
    return base64.urlsafe_b64encode(value).rstrip(b'=').decode('ascii')


def format_key_id(keys_changed_at, key_hash):
    return "{:013d}-{}".format(
        keys_changed_at,
        encode_bytes_b64(key_hash),
    )


def get_fxa_id(databases, user):
    """generate the spanner user key values from the original storage
    data.

    """
    sql = """
        SELECT
            email, generation, keys_changed_at, client_state
        FROM users
            WHERE uid = {uid}
    """.format(uid=user)
    cursor = databases['mysql'].cursor()
    cursor.execute(sql)
    (email, generation, keys_changed_at, client_state) = cursor.next()
    fxa_uid = email.split('@')[0]
    fxa_kid = format_key_id(
        keys_changed_at or generation,
        bytes.fromhex(client_state),
    )
    cursor.close()
    return (fxa_kid, fxa_uid)


def dumper(columns, values):
    """verbose column and data dumper. """
    for row in values:
        print("---\n")
        for i in range(0, len(columns)):
            print("{} => {}".format(columns[i], row[i]))
    print("===\n")


def move_user(databases, user, args):
    """copy user info from original storage to new storage."""
    # bso column mapping:
    # id => bso_id
    # collection => collection_id
    # sortindex => sortindex
    # modified => modified
    # payload => payload
    # payload_size => NONE
    # ttl => expiry

    # user collections require a unique key.
    unique_key_filter = []

    uc_columns = (
        'fxa_kid',
        'fxa_uid',
        'collection_id',
        'modified',
    )
    bso_columns = (
            'collection_id',
            'fxa_kid',
            'fxa_uid',
            'bso_id',
            'expiry',
            'modified',
            'payload',
            'sortindex',
    )

    # Genereate the Spanner Keys we'll need.
    (fxa_kid, fxa_uid) = get_fxa_id(databases, user)

    # Fetch the BSO data from the original storage.
    sql = """
    SELECT collection, id, ttl, modified, payload, sortindex
    FROM bso
    WHERE userid = {}
    ORDER BY modified DESC"""

    count = 0
    with databases['spanner'].batch() as batch:
        cursor = databases['mysql'].cursor()
        try:
            cursor.execute(sql.format(user))
            print("Processing... {} -> {}:{}".format(
                user, fxa_uid, fxa_kid))
            for (cid, bid, exp, mod, pay, sid) in cursor:
                # columns from sync_schema3
                mod_v = datetime.utcfromtimestamp(mod/1000)
                exp_v = datetime.utcfromtimestamp(exp)
                # User_Collection can only have unique values. Filter
                # non-unique keys and take the most recent modified
                # time. The join could be anything.
                uc_key = "{}_{}_{}".format(fxa_uid, fxa_kid, cid)
                if uc_key not in unique_key_filter:
                    unique_key_filter.append(uc_key)
                    uc_values = [(
                        fxa_kid,
                        fxa_uid,
                        cid,
                        mod_v,
                    )]
                    if args.verbose:
                        print("### uc:")
                        dumper(uc_columns, uc_values)
                    batch.insert(
                        'user_collections',
                        columns=uc_columns,
                        values=uc_values
                    )
                # add the BSO values.
                bso_values = [[
                        cid,
                        fxa_kid,
                        fxa_uid,
                        bid,
                        exp_v,
                        mod_v,
                        pay,
                        sid,
                ]]

                if args.verbose:
                    print("###bso:")
                    dumper(bso_columns, bso_values)
                batch.insert(
                    'bsos',
                    columns=bso_columns,
                    values=bso_values
                )
                if databases.get('token'):
                    update_token(databases, user)
                count += 1
                # Closing the with automatically calls `batch.commit()`
        except Exception as e:
            print("### batch failure: {}".format(e))
        finally:
            # cursor may complain about unread data, this should prevent
            # that warning.
            result = cursor.stored_results()
            if args.verbose:
                print("stored results:")
            for ig in result:
                if args.verbose:
                    print("> {}".format(ig))
            if args.verbose:
                print("Closing...")
            cursor.close()
    return count


def move_data(databases, users, args):
    """iterate over provided users and move their data from old to new"""
    for user in users:
        rows = move_user(databases, user.strip(), args)
    return rows


def main():
    start = time.time()
    args = get_args()
    dsns = open(args.dsns).readlines()
    users = open(args.users).readlines()
    databases = {}
    for line in dsns:
        dsn = urlparse(line.strip())
        databases[dsn.scheme] = conf_db(dsn)
    if args.token_dsn:
        dsn = urlparse(args.token_dsn)
        databases['token'] = conf_db(dsn)
    if not databases.get('mysql') or not databases.get('spanner'):
        RuntimeError("Both mysql and spanner dsns must be specified")

    print("Starting:")
    rows = move_data(databases, users, args)
    print("Moved: {} rows in {} seconds".format(rows, time.time() - start))


if __name__ == "__main__":
    main()
