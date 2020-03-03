#! venv/bin/python

# painfully stupid script to check out dumping mysql databases to avro.
# Avro is basically "JSON" for databases. It's not super complicated & it has
# issues (one of which is that it requires Python2).
#
#

import argparse
import logging
import base64
import binascii
import csv
import sys
import math
import json
import os
import time
from datetime import datetime

from mysql import connector
from google.cloud import spanner
from google.api_core.exceptions import AlreadyExists, InvalidArgument
try:
    from urllib.parse import urlparse
except ImportError:
    from urlparse import urlparse

META_GLOBAL_COLLECTION_ID = 6
MAX_ROWS = 1500000


class BadDSNException(Exception):
    pass


class FXA_info:
    """User information from Tokenserver database.

    Can be constructed from
    ``mysql -e "select uid, email, generation, keys_changed_at, \
       client_state from users;" > users.csv`
    """
    users = {}
    anon = False

    def __init__(self, fxa_csv_file, args):
        if args.anon:
            self.anon = True
            return
        logging.debug("Processing token file...")
        if not os.path.isfile(fxa_csv_file):
            raise IOError("{} not found".format(fxa_csv_file))
        with open(fxa_csv_file) as csv_file:
            try:
                line = 0
                for (uid, email, generation,
                     keys_changed_at, client_state) in csv.reader(
                        csv_file, delimiter="\t"):
                    line += 1
                    if uid == 'uid':
                        # skip the header row.
                        continue
                    try:
                        fxa_uid = email.split('@')[0]
                        fxa_kid = self.format_key_id(
                            int(keys_changed_at or generation),
                            binascii.unhexlify(client_state))
                        logging.debug("Adding user {} => {} , {}".format(
                            uid, fxa_kid, fxa_uid
                        ))
                        self.users[int(uid)] = (fxa_kid, fxa_uid)
                    except Exception as ex:
                        logging.error("Skipping user {}:".format(uid), ex)
            except Exception as ex:
                logging.critical("Error in fxa file around line {}: {}".format(
                    line, ex))

    # The following two functions are taken from browserid.utils
    def encode_bytes_b64(self, value):
        return base64.urlsafe_b64encode(value).rstrip(b'=').decode('ascii')

    def format_key_id(self, keys_changed_at, key_hash):
        return "{:013d}-{}".format(
            keys_changed_at,
            self.encode_bytes_b64(key_hash),
        )

    def get(self, userid):
        if userid in self.users:
            return self.users[userid]
        if self.anon:
            fxa_uid = "fake_" + binascii.hexlify(
                os.urandom(11)).decode('utf-8')
            fxa_kid = "fake_" + binascii.hexlify(
                os.urandom(11)).decode('utf-8')
            self.users[userid] = (fxa_kid, fxa_uid)
            return (fxa_kid, fxa_uid)


class Collections:
    """Cache spanner collection list.

    The spanner collection list is the (soon to be) single source of
    truth regarding collection ids.

    """
    _by_name = {
        "clients": 1,
        "crypto": 2,
        "forms": 3,
        "history": 4,
        "keys": 5,
        "meta": 6,
        "bookmarks": 7,
        "prefs": 8,
        "tabs": 9,
        "passwords": 10,
        "addons": 11,
        "addresses": 12,
        "creditcards": 13,
        "reserved": 100,
    }
    spanner = None

    def __init__(self, databases):
        """merge the mysql user_collections into spanner"""
        sql = """
        SELECT
            DISTINCT uc.collection, cc.name
        FROM
            user_collections as uc,
            collections as cc
        WHERE
            uc.collection = cc.collectionid
        ORDER BY
            uc.collection
        """
        cursor = databases['mysql'].cursor()

        def transact(transaction, values):
            transaction.insert(
                'collections',
                columns=('collection_id', 'name'),
                values=values)

        self.spanner = databases['spanner']
        try:
            # fetch existing:
            with self.spanner.snapshot() as scursor:
                rows = scursor.execute_sql(
                    "select collection_id, name from collections")
                for (collection_id, name) in rows:
                    logging.debug("Loading collection: {} => {}".format(
                        name, collection_id
                    ))
                    self._by_name[name] = collection_id
            cursor.execute(sql)
            for (collection_id, name) in cursor:
                if name not in self._by_name:
                    logging.debug("Adding collection: {} => {}".format(
                        name, collection_id
                    ))
                    values = [(collection_id, name)]
                    self._by_name[name] = collection_id
                    # Since a collection may collide, do these one at a time.
                    try:
                        self.spanner.run_in_transaction(transact, values)
                    except AlreadyExists:
                        logging.info(
                            "Skipping already present collection {}".format(
                                values
                            ))
                        pass
        finally:
            cursor.close()

    def get(self, name, collection_id=None):
        """Fetches the collection_id"""

        id = self._by_name.get(name)
        if id is None:
            logging.warn(
                "Unknown collection {}:{} encountered!".format(
                    name, collection_id))
            # it would be swell to add these to the collection table,
            # but that would mean
            # an imbedded spanner transaction, and that's not allowed.
            return None
        return id


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
    if "mysql" in dsn.scheme:
        return conf_mysql(dsn)
    if dsn.scheme == "spanner":
        return conf_spanner(dsn)
    raise RuntimeError("Unknown DSN type: {}".format(dsn.scheme))


def dumper(columns, values):
    """verbose column and data dumper. """
    result = ""
    for row in values:
        for i in range(0, len(columns)):
            result += " {} => {}\n".format(columns[i], row[i])
    return result


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


def divvy(biglist, count):
    lists = []
    biglen = len(biglist)
    start = 0
    while start < biglen:
        lists.append(biglist[start:min(start+count, biglen)])
        start += count
    return lists


def move_user(databases, user, collections, fxa, bso_num, args):
    """copy user info from original storage to new storage."""
    # bso column mapping:
    # id => bso_id
    # collection => collection_id
    # sortindex => sortindex
    # modified => modified
    # payload => payload
    # payload_size => NONE
    # ttl => expiry

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
    try:
        (fxa_kid, fxa_uid) = fxa.get(user)
    except TypeError:
        logging.error("User not found: {} ".format(
            user
        ))
        return 0
    except Exception as ex:
        logging.error(
            "Could not move user: {}".format(user),
            exc_info=ex
        )
        return 0

    # Fetch the BSO data from the original storage.
    sql = """
    SELECT
        collections.name, bso.collection,
        bso.id, bso.ttl, bso.modified, bso.payload, bso.sortindex
    FROM
        bso{} as bso,
        collections
    WHERE
        bso.userid = %s
            and collections.collectionid = bso.collection
            and bso.ttl > unix_timestamp()
    ORDER BY
        modified DESC""".format(bso_num)

    def spanner_transact_uc(
            transaction, data, fxa_kid, fxa_uid, args):
        # user collections require a unique key.
        unique_key_filter = set()
        for (col, cid, bid, exp, mod, pay, sid) in data:
            collection_id = collections.get(col, cid)
            if collection_id is None:
                continue
            # columns from sync_schema3
            mod_v = datetime.utcfromtimestamp(mod/1000.0)
            # User_Collection can only have unique values. Filter
            # non-unique keys and take the most recent modified
            # time. The join could be anything.
            uc_key = "{}_{}_{}".format(fxa_uid, fxa_kid, col)
            if uc_key not in unique_key_filter:
                uc_values = [(
                    fxa_kid,
                    fxa_uid,
                    collection_id,
                    mod_v,
                )]
                if not args.dryrun:
                    transaction.replace(
                        'user_collections',
                        columns=uc_columns,
                        values=uc_values
                    )
                else:
                    logging.debug("not writing {} => {}".format(uc_columns, uc_values))
                unique_key_filter.add(uc_key)

    def spanner_transact_bso(transaction, data, fxa_kid, fxa_uid, args):
        count = 0
        for (col, cid, bid, exp, mod, pay, sid) in data:
            collection_id = collections.get(col, cid)
            if collection_id is None:
                next
            if collection_id != cid:
                logging.debug(
                    "Remapping collection '{}' from {} to {}".format(
                        col, cid, collection_id))
            # columns from sync_schema3
            mod_v = datetime.utcfromtimestamp(mod/1000.0)
            exp_v = datetime.utcfromtimestamp(exp)

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

            if not args.dryrun:
                logging.debug(
                    "###bso{} {}".format(
                        bso_num,
                        dumper(bso_columns, bso_values)
                    )
                )
                transaction.insert(
                    'bsos',
                    columns=bso_columns,
                    values=bso_values
                )
            else:
                logging.debug("not writing {} => {}".format(bso_columns, bso_values))
            count += 1
        return count

    cursor = databases['mysql'].cursor()
    count = 0
    try:
        # Note: cursor() does not support __enter__()
        logging.info("Processing... {} -> {}:{}".format(
            user, fxa_uid, fxa_kid))
        cursor.execute(sql, (user,))
        data = []
        for row in cursor:
            data.append(row)
        for bunch in divvy(data, args.readchunk or 1000):
            # Occasionally, there is a batch fail because a
            # user collection is not found before a bso is written.
            # to solve that, divide the UC updates from the
            # BSO updates.
            # Run through the list of UserCollection updates
            databases['spanner'].run_in_transaction(
                spanner_transact_uc,
                bunch,
                fxa_kid,
                fxa_uid,
                args,
            )
            count += databases['spanner'].run_in_transaction(
                spanner_transact_bso,
                bunch,
                fxa_kid,
                fxa_uid,
                args,
            )

    except AlreadyExists:
        logging.warn(
            "User already imported fxa_uid:{} / fxa_kid:{}".format(
                fxa_uid, fxa_kid
            ))
    except InvalidArgument as ex:
        if "already inserted" in ex.args[0]:
            logging.warn(
                "User already imported fxa_uid:{} / fxa_kid:{}".format(
                    fxa_uid, fxa_kid
                ))
        else:
            raise
    except Exception as e:
        logging.error("### batch failure:", exc_info=e)
    finally:
        # cursor may complain about unread data, this should prevent
        # that warning.
        for result in cursor:
            pass
        cursor.close()
    return count


def move_database(databases, collections, bso_num, fxa, args):
    """iterate over provided users and move their data from old to new"""
    start = time.time()
    cursor = databases['mysql'].cursor()
    # off chance that someone else might have written
    # a new collection table since the last time we
    # fetched.
    rows = 0
    cursor = databases['mysql'].cursor()
    users = []
    if args.user:
        users = [args.user]
    else:
        try:
            sql = """select distinct userid from bso{};""".format(bso_num)
            cursor.execute(sql)
            users = [user for (user,) in cursor]
        except Exception as ex:
            logging.error("Error moving database:", exc_info=ex)
            return rows
        finally:
            cursor.close()
    logging.info("Moving {} users".format(len(users)))
    for user in users:
        rows += move_user(
            databases=databases,
            user=user,
            collections=collections,
            fxa=fxa,
            bso_num=bso_num,
            args=args)
    logging.info("Finished BSO #{} ({} rows) in {} seconds".format(
        bso_num,
        rows,
        math.ceil(time.time() - start)
    ))
    return rows


def get_args():
    parser = argparse.ArgumentParser(
        description="move user from sql to spanner")
    parser.add_argument(
        '--dsns', default="move_dsns.lst",
        help="file of new line separated DSNs")
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
        '--chunk_limit', type=int, default=1500000,
        dest='limit',
        help="Limit each read chunk to n rows")
    parser.add_argument(
        '--offset', type=int, default=0,
        help="UID to start at")
    parser.add_argument(
        "--full",
        action="store_true",
        help="force a full reconcile"
    )
    parser.add_argument(
        '--deanon', action='store_false',
        dest='anon',
        help="Do not anonymize the user data"
    )
    parser.add_argument(
        '--start_bso', default=0,
        type=int,
        help="start dumping BSO database"
    )
    parser.add_argument(
        '--end_bso',
        type=int, default=19,
        help="last BSO database to dump"
    )
    parser.add_argument(
        '--fxa_file',
        default="users.csv",
        help="FXA User info in CSV format"
    )
    parser.add_argument(
        '--skip_collections', action='store_false',
        help="skip user_collections table"
    )
    parser.add_argument(
        '--readchunk',
        default=1000,
        help="how many rows per transaction for spanner"
    )
    parser.add_argument(
        '--user',
        type=str,
        help="BSO#:userId to move (EXPERIMENTAL)."
    )
    parser.add_argument(
        '--dryrun',
        action="store_true",
        help="Do not write user records to spanner."
    )


    return parser.parse_args()


def main():
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
    databases = {}
    rows = 0

    if args.user:
        (bso, userid) = args.user.split(':')
        args.start_bso = int(bso)
        args.end_bso = int(bso)
        args.user = int(userid)
    for line in dsns:
        dsn = urlparse(line.strip())
        scheme = dsn.scheme
        if 'mysql' in dsn.scheme:
            scheme = 'mysql'
        databases[scheme] = conf_db(dsn)
    if not databases.get('mysql') or not databases.get('spanner'):
        RuntimeError("Both mysql and spanner dsns must be specified")
    fxa_info = FXA_info(args.fxa_file, args)
    collections = Collections(databases)
    logging.info("Starting:")
    if args.dryrun:
        logging.info("=== DRY RUN MODE ===")
    start = time.time()
    for bso_num in range(args.start_bso, args.end_bso+1):
        logging.info("Moving users in bso # {}".format(bso_num))
        rows += move_database(
            databases, collections, bso_num, fxa_info, args)
    logging.info(
        "Moved: {} rows in {} seconds".format(
            rows or 0, time.time() - start))


if __name__ == "__main__":
    main()
