#! venv/bin/python

# This file is historical.
# This file will attempt to copy a user from an existing mysql database
# to a spanner table. It requires access to the tokenserver db, which may
# not be available in production environments.
#
#

import argparse
import logging
import base64

import sys
import os
import time
from datetime import datetime

from mysql import connector
from mysql.connector.errors import IntegrityError
from google.cloud import spanner
from google.api_core.exceptions import AlreadyExists
try:
    from urllib.parse import urlparse
except ImportError:
    from urlparse import urlparse

SPANNER_NODE_ID = 800
META_GLOBAL_COLLECTION_ID = 6

class BadDSNException(Exception):
    pass


# From server_syncstorage
class MigrationState:
    UKNOWN = 0
    IN_PROGRESS = 1
    COMPLETE = 2


class Collections:
    """Cache spanner collection list.

    The spanner collection list is the (soon to be) single source of
    truth regarding collection ids.

    """
    _by_name = {}
    databases = None

    def __init__(self, databases):
        """Get the cache list of collection ids"""
        sql = """
        SELECT
            name, collection_id
        FROM
            collections;
        """
        self.databases = databases
        logging.debug("Fetching collections...")
        with self.databases['spanner'].snapshot() as cursor:
            rows = cursor.execute_sql(sql)
            for row in rows:
                self._by_name[row[0]] = row[1]

    def get_id(self, name, cursor):
        """ Get/Init the ID for a given collection """
        if name in self._by_name:
            return self._by_name.get(name)
        result = cursor.execute_sql("""
            SELECT
                COALESCE(MAX(collection_id), 1)
            FROM
                collections""")
        # preserve the "reserved" / < 100 ids.
        collection_id = max(result.one()[0] + 1, 101)
        cursor.insert(
            table="collections",
            columns=('collection_id', 'name'),
            values=[
                (collection_id, name)
            ]
        )
        self._by_name[name] = collection_id
        return collection_id


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
    parser.add_argument(
        '--quiet',
        action="store_true",
        help="silence logging"
    )
    parser.add_argument(
        "--full",
        action="store_true",
        help="force a full reconcile"
    )
    return parser.parse_args()


def conf_mysql(dsn):
    """create a connection to the original storage system """
    logging.debug("Configuring MYSQL: {}".format(dsn))
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
    logging.debug("Configuring SPANNER: {}".format(dsn))
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
    if 'token' not in databases:
        logging.warn(
            "Skipping token update for user {}...".format(user))
        return
    logging.info("Updating token server for user: {}".format(user))
    try:
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
        databases['token'].commit()
    finally:
        cursor.close()


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
            email, generation, keys_changed_at, client_state, node
        FROM users
            WHERE uid = {uid}
    """.format(uid=user)
    try:
        cursor = databases.get('token', databases['mysql']).cursor()
        cursor.execute(sql)
        (email, generation, keys_changed_at,
         client_state, node) = cursor.next()
        fxa_uid = email.split('@')[0]
        fxa_kid = format_key_id(
            keys_changed_at or generation,
            bytes.fromhex(client_state),
        )
    finally:
        cursor.close()
    return (fxa_kid, fxa_uid, node)


def create_migration_table(database):
    """create the syncstorage table

    This table tells the syncstorage server to return a 5xx for a
    given user. It's important that syncstorage NEVER returns a
    2xx result for any user that's in migration, or only does
    so after deleting the meta/global BSO record so that a full
    reconcile happens. (Depends on
    https://github.com/mozilla-services/server-syncstorage/pull/136)
    """
    try:
        cursor = database.cursor()
        cursor.execute(
            """CREATE TABLE IF NOT EXISTS
                migration (
                    fxa_uid VARCHAR(255) NOT NULL PRIMARY KEY,
                    started_at BIGINT NOT NULL,
                    state SMALLINT
                )
            """)
        database.commit()
    finally:
        cursor.close()


def dumper(columns, values):
    """verbose column and data dumper. """
    result = ""
    for row in values:
        for i in range(0, len(columns)):
            result += " {} => {}\n".format(columns[i], row[i])
    return result


def mark_user(databases, user, state=MigrationState.IN_PROGRESS):
    """ mark a user in migration """
    try:
        mysql = databases['mysql'].cursor()
        if state == MigrationState.IN_PROGRESS:
            try:
                logging.info("Marking {} as migrating...".format(user))
                mysql.execute(
                    "INSERT INTO migration "
                    "(fxa_uid, started, state) VALUES (%s, %s, %s)",
                    (user, int(time.time()), state)
                )
                databases['mysql'].commit()
            except IntegrityError:
                return False
        if state == MigrationState.COMPLETE:
            logging.info("Marking {} as migrating...".format(user))
            mysql.execute(
                "UPDATE migration SET state = %s WHERE fxa_uid = %s",
                (state, user)
            )
            databases['mysql'].commit()
    finally:
        mysql.close()
    return True


def finish_user(databases, user):
    """mark a user migration complete"""
    # This is not wrapped into `start_user` so that I can reduce
    # the number of db IO, since an upsert would just work instead
    # of fail out with a dupe.
    mysql = databases['mysql'].cursor()
    try:
        logging.info("Marking {} as migrating...".format(user))
        mysql.execute(
            """
            UPDATE
                migration
            SET
                state = "finished"
            WHERE
                fxa_uid = %s
            """,
            (user,)
        )
        databases['mysql'].commit()
    except IntegrityError:
        return False
    finally:
        mysql.close()
    return True

def newSyncID():
    base64.urlsafe_b64encode(os.urandom(9))

def alter_syncids(pay):
    """Alter the syncIDs for the meta/global record, which will cause a sync
    when the client reconnects


    """
    payload = json.loads(pay)
    payload['syncID'] = newSyncID()
    for item in payload['engines']:
        payload['engines'][item]['syncID'] = newSyncID()
    return json.dumps(payload)

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
    unique_key_filter = set()

    # off chance that someone else might have written
    # a new collection table since the last time we
    # fetched.
    collections = Collections(databases)

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
    (fxa_kid, fxa_uid, original_node) = get_fxa_id(databases, user)
    if not start_user(databases, fxa_uid):
        logging.error("User {} already being migrated?".format(fxa_uid))
        return

    # Fetch the BSO data from the original storage.
    sql = """
    SELECT
        collections.name, bso.collection,
        bso.id, bso.ttl, bso.modified, bso.payload, bso.sortindex
    FROM
        collections, bso
    WHERE
        bso.userid = %s and collections.collectionid = bso.collection
    ORDER BY
        modified DESC"""

    count = 0

    def spanner_transact(transaction):
        collection_id = collections.get_id(col, transaction)
        if collection_id != cid:
            logging.warn(
                "Remapping collection '{}' from {} to {}".format(
                    col, cid, collection_id))
        # columns from sync_schema3
        mod_v = datetime.utcfromtimestamp(mod/1000.0)
        exp_v = datetime.utcfromtimestamp(exp)
        # User_Collection can only have unique values. Filter
        # non-unique keys and take the most recent modified
        # time. The join could be anything.
        uc_key = "{}_{}_{}".format(fxa_uid, fxa_kid, col)
        if uc_key not in unique_key_filter:
            unique_key_filter.add(uc_key)
            uc_values = [(
                fxa_kid,
                fxa_uid,
                collection_id,
                mod_v,
            )]
            logging.debug(
                "### uc: {}".format(uc_columns, uc_values))
            transaction.insert(
                'user_collections',
                columns=uc_columns,
                values=uc_values
            )
        # add the BSO values.
        if args.full and collection_id == META_GLOBAL_COLLECTION_ID:
            pay = alter_syncids(pay)
        bso_values = [[
                collection_id,
                fxa_kid,
                fxa_uid,
                bid,
                exp_v,
                mod_v,
                pay,
                sid,
        ]]

        logging.debug(
            "###bso: {}".format(dumper(bso_columns, bso_values)))
        transaction.insert(
            'bsos',
            columns=bso_columns,
            values=bso_values
        )
    mysql = databases['mysql'].cursor()
    try:
        # Note: cursor() does not support __enter__()
        mysql.execute(sql, (user,))
        logging.info("Processing... {} -> {}:{}".format(
            user, fxa_uid, fxa_kid))
        for (col, cid, bid, exp, mod, pay, sid) in mysql:
            databases['spanner'].run_in_transaction(spanner_transact)
            update_token(databases, user)
            (ck_kid, ck_uid, ck_node) = get_fxa_id(databases, user)
            if ck_node != original_node:
                logging.error(
                    ("User's Node Changed! Aborting! "
                    "fx_uid:{}, fx_kid:{}, node: {} => {}")
                    .format(user, fxa_uid, fxa_kid,
                            original_node, ck_node)
                )
                return
            finish_user(databases, user)
            count += 1
            # Closing the with automatically calls `batch.commit()`
        mark_user(user, MigrationState.COMPLETE)
    except AlreadyExists:
        logging.warn(
            "User already imported fxa_uid:{} / fxa_kid:{}".format(
                fxa_uid, fxa_kid
            ))
    except Exception as e:
        logging.error("### batch failure:", e)
    finally:
        # cursor may complain about unread data, this should prevent
        # that warning.
        for result in mysql:
            pass
        logging.debug("Closing...")
        mysql.close()
    return count


def move_data(databases, users, args):
    """iterate over provided users and move their data from old to new"""
    for user in users:
        rows = move_user(databases, user.strip(), args)
    return rows


def main():
    start = time.time()
    args = get_args()
    log_level = logging.INFO
    if args.quiet:
        log_level = logging.ERROR
    if args.verbose:
        log_level = logging.DEBUG
    logging.basicConfig(
        stream=sys.stdout,
        level=log_level,
    )
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

    # create the migration table if it's not already present.
    # This table is used by the sync storage server to force a 500 return
    # for a user in migration.
    create_migration_table(databases['mysql'])

    logging.info("Starting:")
    rows = move_data(databases, users, args)
    logging.info(
        "Moved: {} rows in {} seconds".format(
            rows or 0, time.time() - start))


if __name__ == "__main__":
    main()
