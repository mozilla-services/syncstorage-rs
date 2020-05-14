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

import grpc
from mysql import connector
from google.cloud import spanner
from google.cloud.spanner_v1 import param_types
from google.api_core.exceptions import AlreadyExists, InvalidArgument
try:
    from urllib.parse import urlparse
except ImportError:
    from urlparse import urlparse

META_GLOBAL_COLLECTION_NAME = "meta"
MAX_ROWS = 1500000


class BadDSNException(Exception):
    pass


def tick(count):
    mark = None
    if not count % 100:
        mark = "."
    if not count % 1000:
        mark = "|"
    level = logging.getLogger().getEffectiveLevel()
    if mark and level > logging.DEBUG:
        print(mark, end='', flush=True)


class Report:

    bso = "init"
    _success = None
    _failure = None

    def __init__(self, args):
        self._success_file = args.success_file
        self._failure_file = args.failure_file

    def success(self, uid):
        if not self._success:
            self._success = open(self._success_file, "w")
        self._success.write("{}\t{}\n".format(self.bso, uid))

    def fail(self, uid, reason=None):
        if not self._failure:
            self._failure = open(self._failure_file, "w")
        logging.debug("Skipping user {}".format(uid))
        self._failure.write("{}\t{}\t{}\n".format(self.bso, uid, reason or ""))

    def close(self):
        self._success.close()
        self._failure.close()


class FXA_info:
    """User information from Tokenserver database.

    Can be constructed from
    ``mysql -e "select uid, email, generation, keys_changed_at, \
       client_state from users;" > users.csv`
    """
    users = {}
    anon = False

    def __init__(self, users_file, args, report):
        if args.anon:
            self.anon = True
            return
        logging.info("Reading users file: {}".format(users_file))
        if not os.path.isfile(users_file):
            raise IOError("{} not found".format(users_file))
        with open(users_file) as csv_file:
            try:
                line = 0
                for (uid, fxa_uid, fxa_kid) in csv.reader(
                        csv_file, delimiter="\t"):
                    line += 1
                    tick(line)
                    if uid == 'uid':
                        # skip the header row.
                        continue
                    if args.user:
                        if int(uid) not in args.user:
                            continue
                    try:
                        self.users[int(uid)] = (fxa_kid, fxa_uid)
                    except Exception as ex:
                        logging.error(
                            "User {} Unexpected error".format(uid),
                            exc_info=ex)
                        report.fail(uid, "unexpected error")
            except Exception as ex:
                logging.critical("Error in fxa file around line {}".format(
                    line), exc_info=ex)

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
    if "spanner" in dsn.scheme:
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
    """Partition a list into a set of equally sized slices"""
    lists = []
    biglen = len(biglist)
    start = 0
    while start < biglen:
        lists.append(biglist[start:min(start+count, biglen)])
        start += count
    return lists


def move_user(databases, user_data, collections, fxa, bso_num, args, report):
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
        'fxa_uid',
        'fxa_kid',
        'collection_id',
        'modified',
    )
    bso_columns = (
            'collection_id',
            'fxa_uid',
            'fxa_kid',
            'bso_id',
            'expiry',
            'modified',
            'payload',
            'sortindex',
    )

    (uid, fxa_uid, fxa_kid) = user_data
    # Fetch the BSO data from the original storage.
    sql = """
    SELECT
        collections.name, bso.collection, uc.last_modified,
        bso.id, bso.ttl, bso.modified, bso.payload, bso.sortindex
    FROM
        bso{} as bso,
        collections,
        user_collections as uc
    WHERE
        bso.userid = %s
            and collections.collectionid = bso.collection
            and uc.collection = bso.collection
            and uc.userid = bso.userid
            and bso.ttl > unix_timestamp()
    ORDER BY
        bso.collection, bso.id""".format(bso_num)
    unique_key_filter = set()

    def spanner_transact_wipe_user(
            transaction, fxa_uid, fxa_kid, args):
        result = transaction.execute_sql(
            """
        SELECT
            uc.collection_id, c.name
        FROM
            user_collections as uc
        LEFT JOIN
            collections as c
        ON
            uc.collection_id = c.collection_id
        WHERE
            uc.fxa_uid = @fxa_uid
        AND uc.fxa_kid = @fxa_kid
            """,
            params=dict(fxa_uid=fxa_uid, fxa_kid=fxa_kid),
            param_types=dict(fxa_uid=param_types.STRING, fxa_kid=param_types.STRING),
        )
        cols = [(row[0], row[1]) for row in result]
        if not args.dryrun:
            logging.debug("Wiping user, collections: {}".format(cols))
            transaction.execute_update(
                """
            DELETE FROM
                user_collections
            WHERE
                fxa_uid = @fxa_uid
            AND fxa_kid = @fxa_kid
            """,
                params=dict(fxa_uid=fxa_uid, fxa_kid=fxa_kid),
                param_types=dict(fxa_uid=param_types.STRING, fxa_kid=param_types.STRING),
            )
        else:
            logging.debug("Not wiping user, collections: {}".format(cols))

    def spanner_transact_uc(
            transaction, data, fxa_uid, fxa_kid, args):
        # user collections require a unique key.
        for (col, cid, cmod, bid, exp, bmod, pay, sid) in data:
            collection_id = collections.get(col, cid)
            if collection_id is None:
                continue
            # columns from sync_schema3
            # user_collections modified should come directly from
            # mysql user_collections.last_modified
            mod_v = datetime.utcfromtimestamp(cmod/1000.0)
            # User_Collection can only have unique values. Filter
            # non-unique keys and take the most recent modified
            # time. The join could be anything.
            uc_key = "{}_{}_{}".format(fxa_uid, fxa_kid, col)
            if uc_key not in unique_key_filter:
                uc_values = [(
                    fxa_uid,
                    fxa_kid,
                    collection_id,
                    mod_v,
                )]
                if not args.dryrun:
                    transaction.insert(
                        'user_collections',
                        columns=uc_columns,
                        values=uc_values
                    )
                else:
                    logging.debug("not writing {} => {}".format(
                        uc_columns, uc_values))
                unique_key_filter.add(uc_key)

    def spanner_transact_bso(transaction, data, fxa_uid, fxa_kid, args):
        count = 0
        bso_values = []
        for (col, cid, cmod, bid, exp, bmod, pay, sid) in data:
            collection_id = collections.get(col, cid)
            if collection_id is None:
                continue
            if collection_id != cid:
                logging.debug(
                    "Remapping collection '{}' from {} to {}".format(
                        col, cid, collection_id))
            # columns from sync_schema3
            mod_v = datetime.utcfromtimestamp(bmod/1000.0)
            exp_v = datetime.utcfromtimestamp(exp)

            # add the BSO values.
            if args.full and col == META_GLOBAL_COLLECTION_NAME:
                pay = alter_syncids(pay)
            bso_values.append([
                    collection_id,
                    fxa_uid,
                    fxa_kid,
                    bid,
                    exp_v,
                    mod_v,
                    pay,
                    sid,
            ])

            count += 1
        if not args.dryrun:
            logging.debug(
                "###bso{} {}".format(
                    bso_num,
                    dumper(bso_columns, bso_values)
                )
            )
            for i in range(0, 5):
                try:
                    transaction.insert(
                        'bsos',
                        columns=bso_columns,
                        values=bso_values
                    )
                    break
                except grpc._channel_._InactiveRpcError as ex:
                    logging.warn(
                        "Could not write record (attempt {})".format(i),
                        exc_info=ex)
                    time.sleep(.5)
        else:
            logging.debug("not writing {} => {}".format(
                bso_columns, bso_values))
        return count

    cursor = databases['mysql'].cursor()
    count = 0
    try:
        # Note: cursor() does not support __enter__()
        logging.info("Processing... {} -> {}:{}".format(
            uid, fxa_uid, fxa_kid))
        cursor.execute(sql, (uid,))
        data = []
        abort_col = None
        abort_count = None
        col_count = 0

        if args.abort:
            (abort_col, abort_count) = args.abort.split(":")
            abort_count = int(abort_count)
        for row in cursor:
            logging.debug("col: {}".format(row[0]))
            if abort_col and int(row[1]) == int(abort_col):
                col_count += 1
                if col_count > abort_count:
                    logging.debug("Skipping col: {}: {} of {}".format(
                        row[0], col_count, abort_count))
                    continue
            data.append(row)
        if args.abort:
            logging.info("Skipped {} of {} rows for {}".format(
                abort_count, col_count, abort_col
            ))
        logging.info(
            "Moving {} items for user {} => {}:{}".format(
                len(data), uid, fxa_uid, fxa_kid))

        if args.wipe_user:
            databases['spanner'].run_in_transaction(
                spanner_transact_wipe_user,
                fxa_uid,
                fxa_kid,
                args,
            )

        for bunch in divvy(data, args.chunk or 1000):
            # Occasionally, there is a batch fail because a
            # user collection is not found before a bso is written.
            # to solve that, divide the UC updates from the
            # BSO updates.
            # Run through the list of UserCollection updates
            databases['spanner'].run_in_transaction(
                spanner_transact_uc,
                bunch,
                fxa_uid,
                fxa_kid,
                args,
            )
            count += databases['spanner'].run_in_transaction(
                spanner_transact_bso,
                bunch,
                fxa_uid,
                fxa_kid,
                args,
            )
            if args.ms_delay > 0:
                logging.debug(
                    "Sleeping for {} seconds".format(args.ms_delay * .01))
                time.sleep(args.ms_delay * .01)

    except AlreadyExists:
        logging.warn(
            "User {} already imported fxa_uid:{} / fxa_kid:{}".format(
                uid, fxa_uid, fxa_kid
            ))
        report.fail(uid, "exists")
        return count
    except InvalidArgument as ex:
        report.fail(uid, "exists")
        if "already inserted" in ex.args[0]:
            logging.warn(
                "User {} already imported fxa_uid:{} / fxa_kid:{}".format(
                    uid, fxa_uid, fxa_kid
                ))
            return count
        else:
            raise
    except Exception as ex:
        report.fail(uid, "unexpected batch error")
        logging.error("Unexpected Batch failure: {}:{}".format(
            fxa_uid, fxa_kid), exc_info=ex)
    finally:
        # cursor may complain about unread data, this should prevent
        # that warning.
        for result in cursor:
            pass
        cursor.close()
    report.success(uid)
    return count


def get_percentage_users(users, user_percent):
    (block, percentage) = map(
        int, user_percent.split(':'))
    total_count = len(users)
    chunk_size = max(
        1, math.floor(
            total_count * (int(percentage) * .01)))
    chunk_count = math.ceil(total_count / chunk_size)
    chunk_start = max(block - 1, 0) * chunk_size
    chunk_end = min(chunk_count, block) * chunk_size
    if chunk_size * chunk_count > total_count:
        if block >= chunk_count - 1:
            chunk_end = total_count
    users = users[chunk_start:chunk_end]
    logging.debug(
        "moving users: {} to {}".format(
            chunk_start, chunk_end))
    return users


def get_users(args, databases, fxa, bso_num, report):
    """Fetch the user information from the Tokenserver Dump """
    users = []
    try:
        if args.user:
            for uid in args.user:
                try:
                    (fxa_kid, fxa_uid) = fxa.get(uid)
                    users.append((uid, fxa_uid, fxa_kid))
                except TypeError:
                    logging.error(
                        "User {} not found in "
                        "tokenserver data.".format(uid))
                    report.fail(uid, "not found")
        else:
            try:
                bso_users_file = args.bso_users_file.replace('#', str(bso_num))
                with open(bso_users_file) as bso_file:
                    line = 0
                    for row in csv.reader(
                        bso_file, delimiter="\t"
                    ):
                        if row[0] == "uid":
                            continue
                        users.append(row)
                        tick(line)
                        line += 1
            except Exception as ex:
                logging.critical("Error reading BSO data", exc_info=ex)
                exit(-1)
            if args.user_percent:
                users = get_percentage_users(users, args.user_percent)
    except Exception as ex:
        logging.critical("Unexpected Error moving database:", exc_info=ex)
        exit(-1)
    return users


def move_database(databases, collections, bso_num, fxa, args, report):
    """iterate over provided users and move their data from old to new"""
    start = time.time()
    # off chance that someone else might have written
    # a new collection table since the last time we
    # fetched.
    rows = 0
    users = get_users(args, databases, fxa, bso_num, report)
    logging.info("Moving {} users".format(len(users)))
    for user in users:
        rows += move_user(
            databases=databases,
            user_data=user,
            collections=collections,
            fxa=fxa,
            bso_num=bso_num,
            args=args,
            report=report)
    logging.info("Finished BSO #{} ({} rows) in {} seconds".format(
        bso_num,
        rows,
        math.ceil(time.time() - start)
    ))
    return rows


def get_args():
    pid = os.getpid()
    today = datetime.now().strftime("%Y_%m_%d")
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
        '--offset', type=int, default=0,
        help="UID to start at (default 0)")
    parser.add_argument(
        "--full",
        action="store_true",
        help="force a full reconcile"
    )
    parser.add_argument(
        '--anon', action='store_true',
        help="Anonymize the user data"
    )
    parser.add_argument(
        '--start_bso', default=0,
        type=int,
        help="start dumping BSO database (default: 0)"
    )
    parser.add_argument(
        '--end_bso',
        type=int, default=19,
        help="last BSO database to dump (default: 19)"
    )
    parser.add_argument(
        '--bso_num',
        type=int,
        help="only move this bso (equivalent to start_bso == end_bso)"
    )
    parser.add_argument(
        '--write_chunk',
        dest="chunk",
        default=1666,
        help="how many rows per transaction for spanner (default: 1666)"
    )
    parser.add_argument(
        '--user',
        type=str,
        help="BSO#:userId[,userid,...] to move."
    )
    parser.add_argument(
        '--wipe_user',
        action="store_true",
        help="delete any pre-existing --user data on spanner before the migration"
    )
    parser.add_argument(
        '--bso_users_file',
        default="bso_users_#_{}.lst".format(today),
        help="name of the generated BSO user file. "
            "(Will use bso number for `#` if present; "
            "default: bso_users_#_{}.lst)".format(today),
    )
    parser.add_argument(
        '--fxa_users_file',
        default="fxa_users_{}.lst".format(today),
        help="List of pre-generated FxA users. Only needed if specifying"
            " the `--user` option; default: fxa_users_{}.lst)".format(today)
    )
    parser.add_argument(
        '--dryrun',
        action="store_true",
        help="Do not write user records to spanner"
    )
    parser.add_argument(
        '--abort',
        type=str,
        help="abort data in col after #rows (e.g. history:10)"
    )
    parser.add_argument(
        "--user_percent", default="1:100",
        help=("Offset and percent of users from this BSO"
              "to move (e.g. 2:50 moves the second 50%%) "
              "(default 1:100)")
    )
    parser.add_argument(
        '--ms_delay', type=int, default=0,
        help="inject a sleep between writes to spanner as a throttle"
    )
    parser.add_argument(
        '--success_file', default="success_{}.log".format(pid),
        help="File of successfully migrated userids"
    )
    parser.add_argument(
        '--failure_file', default="failure_{}.log".format(pid),
        help="File of unsuccessfully migrated userids"
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
    report = Report(args)
    dsns = open(args.dsns).readlines()
    databases = {}
    rows = 0

    if args.user:
        args.user_percent = "1:100"
        (bso, userid) = args.user.split(':')
        args.start_bso = int(bso)
        args.end_bso = int(bso)
        user_list = []
        for id in userid.split(','):
            user_list.append(int(id))
        args.user = user_list
    elif args.wipe_user:
        raise RuntimeError("--wipe_user requires --user")
    if args.bso_num is not None:
        args.start_bso = args.end_bso = args.bso_num
    for line in dsns:
        dsn = urlparse(line.strip())
        scheme = dsn.scheme
        if 'mysql' in dsn.scheme:
            scheme = 'mysql'
        databases[scheme] = conf_db(dsn)
    if not databases.get('mysql') or not databases.get('spanner'):
        raise RuntimeError("Both mysql and spanner dsns must be specified")
    fxa_info = FXA_info(args.fxa_users_file, args, report)
    collections = Collections(databases)
    logging.info("Starting:")
    if args.dryrun:
        logging.info("=== DRY RUN MODE ===")
    start = time.time()
    for bso_num in range(args.start_bso, args.end_bso+1):
        logging.info("Moving users in bso # {}".format(bso_num))
        report.bso = bso_num
        rows += move_database(
            databases, collections, bso_num, fxa_info, args, report)
    logging.info(
        "Moved: {} rows in {} seconds".format(
            rows or 0, time.time() - start))


if __name__ == "__main__":
    main()
