#! venv/bin/python
#

import argparse
import logging
import csv
import sys
import os
from datetime import datetime

from mysql import connector
try:
    from urllib.parse import urlparse
except ImportError:
    from urlparse import urlparse


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
    _failure = None
    _success = None

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


class BSO_Users:
    """User information from Tokenserver database.

    Can be constructed from
    ``mysql -e "select uid, email, generation, keys_changed_at, \
       client_state from users;" > users.csv`
    """
    users = {}
    anon = False

    def __init__(self, args, report, dsn):
        self.args = args
        self.report = report
        self.conf_mysql(dsn)
        self.get_users(args)

    def get_users(self, args):
        try:
            logging.info("Reading fxa_user data.")
            with open(args.fxa_users_file) as csv_file:
                line = 0
                for (uid, fxa_uid, fxa_kid) in csv.reader(
                    csv_file, delimiter="\t"
                ):
                    if uid == "uid":
                        continue
                    self.tick(line)
                    logging.debug("Read: {} {}:{}".format(
                        uid, fxa_uid, fxa_kid))
                    self.users[int(uid)] = (fxa_uid, fxa_kid)
                    line += 1
        except Exception as ex:
            logging.error(
                "Unexpected error",
                exc_info=ex
            )
            self.report.fail(uid, "Unexpected error {}".format(ex))

    def run(self):
        out_users = []
        logging.info("Fetching users from BSO dbinto {}".format(
            self.args.bso_users_file,
        ))
        output_file = open(self.args.bso_users_file, "w")
        try:
            cursor = self.connection.cursor()
            sql = ("""select userid, count(*) as count from bso{}"""
                   """ group by userid order by userid""".format(
                       self.args.bso_num))
            if self.args.user_range:
                (offset, limit) = self.args.user_range.split(':')
                sql = "{} limit {} offset {}".format(
                    sql, limit, offset)
            cursor.execute(sql)
            for (uid, count) in cursor:
                try:
                    (fxa_uid, fxa_kid) = self.users.get(uid)
                    if self.args.hoard_limit and count > self.args.hoard_limit:
                        logging.warn(
                            "User {} => {}:{} has too "
                            "many items: {} ".format(
                                uid, fxa_uid, fxa_kid, count
                            )
                        )
                        self.report.fail(uid, "hoarder {}".format(count))
                        continue
                    out_users.append((uid, fxa_uid, fxa_kid))
                except TypeError:
                    self.report.fail(uid, "not found")
                    logging.error(
                        ("User {} not found in "
                            "tokenserver data".format(uid)))
            if self.args.sort_users:
                out_users.sort(key=lambda tup: tup[1])
            # Take a block of percentage of the users.
            logging.info("Writing out {} users".format(len(out_users)))
            line = 0
            output_file.write("uid\tfxa_uid\tfxa_kid\n")
            for (uid, fxa_uid, fxa_kid) in out_users:
                output_file.write("{}\t{}\t{}\n".format(
                    uid, fxa_uid, fxa_kid
                ))
                tick(line)
        finally:
            output_file.flush()
            cursor.close()

    def conf_mysql(self, dsn):
        """create a connection to the original storage system """
        logging.debug("Configuring MYSQL: {}".format(dsn))
        self.connection = connector.connect(
            user=dsn.username,
            password=dsn.password,
            host=dsn.hostname,
            port=dsn.port or 3306,
            database=dsn.path[1:]
        )
        return self.connection


def get_args():
    pid = os.getpid()
    parser = argparse.ArgumentParser(
        description="Generate BSO user list")
    parser.add_argument(
        '--dsns', default="move_dsns.lst",
        help="file of new line separated DSNs")
    parser.add_argument(
        '--bso_num',
        default="0",
        help="BSO database number"
    )
    parser.add_argument(
        '--bso_users_file',
        default="bso_users_{}_{}.lst".format(
            0, datetime.now().strftime("%Y_%m_%d")),
        help="List of BSO users."
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
        '--user_range',
        help="Range of users to extract (offset:limit)"
    )
    parser.add_argument(
        '--hoard_limit', type=int, default=0,
        help="reject any user with more than this count of records"
    )
    parser.add_argument(
        '--sort_users', action="store_true",
        help="Sort the user"
        )
    parser.add_argument(
        '--success_file', default="success_bso_user.log".format(pid),
        help="File of successfully migrated userids"
    )
    parser.add_argument(
        '--failure_file', default="failure_bso_user.log".format(pid),
        help="File of unsuccessfully migrated userids"
    )
    parser.add_argument(
        '--fxa_users_file',
        default="fxa_users_{}.lst".format(datetime.now().strftime("%Y_%m_%d")),
        help="List of pre-generated FxA users."
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
    db_dsn = None
    for line in dsns:
        dsn = urlparse(line.strip())
        if 'mysql' in dsn.scheme:
            db_dsn = dsn

    if not db_dsn:
        RuntimeError("mysql dsn must be specified")

    bso = BSO_Users(args, report, db_dsn)
    bso.run()


if __name__ == "__main__":
    main()
