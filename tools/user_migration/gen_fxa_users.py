#! venv/bin/python
#

import argparse
import logging
import base64
import binascii
import csv
import sys
import os
from datetime import datetime


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


class FxA_Generate:
    """User information from Tokenserver database.

    Can be constructed from
    ``mysql -e "select uid, email, generation, keys_changed_at, \
       client_state from users;" > users.csv`
    """
    users = []
    anon = False

    def __init__(self, args, report):
        logging.info("Processing token file: {} into {}".format(
            args.fxa_file,
            args.fxa_users_file,
        ))
        output_file = open(args.fxa_users_file, "w")
        output_file.write("uid\tfxa_uid\tfxa_kid\n")
        if not os.path.isfile(args.fxa_file):
            raise IOError("{} not found".format(args.fxa_file))
        with open(args.fxa_file) as csv_file:
            try:
                line = 0
                for (uid, email, generation,
                     keys_changed_at, client_state) in csv.reader(
                        csv_file, delimiter="\t"):
                    line += 1
                    if uid == 'uid':
                        # skip the header row.
                        continue
                    tick(line)
                    try:
                        fxa_uid = email.split('@')[0]
                        try:
                            keys_changed_at = int(keys_changed_at)
                        except ValueError:
                            keys_changed_at = 0

                        try:
                            generation = int(generation)
                        except ValueError:
                            generation = 0

                        if (keys_changed_at or generation) == 0:
                            logging.warn(
                                "user {} has no k_c_a or "
                                "generation value".format(
                                    uid))
                        try:
                            client_state = binascii.unhexlify(client_state)
                        except binascii.Error:
                            logging.error(
                                "User {} has "
                                "invalid client state: {}".format(
                                    uid, client_state
                                ))
                            report.fail(uid, "bad client state")
                            continue
                        fxa_kid = self.format_key_id(
                            int(keys_changed_at or generation),
                            client_state
                            )
                        logging.debug("Adding user {} => {} , {}".format(
                            uid, fxa_uid, fxa_kid
                        ))
                        output_file.write(
                            "{}\t{}\t{}\n".format(
                                uid, fxa_uid, fxa_kid))
                    except Exception as ex:
                        logging.error(
                            "User {} Unexpected error".format(uid),
                            exc_info=ex)
                        report.fail(uid, "unexpected error")
            except Exception as ex:
                logging.critical("Error in fxa file around line {}".format(
                    line), exc_info=ex)

    # The following two functions are taken from browserid.utils
    def encode_bytes_b64(self, value):
        return base64.urlsafe_b64encode(value).rstrip(b'=').decode('ascii')

    def format_key_id(self, keys_changed_at, key_hash):
        return "{:013d}-{}".format(
            keys_changed_at,
            self.encode_bytes_b64(key_hash),
        )


def get_args():
    pid = os.getpid()
    parser = argparse.ArgumentParser(
        description="Generate FxA user id info")
    parser.add_argument(
        '--fxa_file',
        default="users.csv",
        help="FXA User info in CSV format (default users.csv)"
    )
    parser.add_argument(
        '--fxa_users_file',
        default="fxa_users_{}.lst".format(datetime.now().strftime("%Y_%m_%d")),
        help="List of FxA users."
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
        '--success_file', default="success_fxa_user.log".format(pid),
        help="File of successfully migrated userids"
    )
    parser.add_argument(
        '--failure_file', default="failure_fxa_user.log".format(pid),
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
    FxA_Generate(args, report)


if __name__ == "__main__":
    main()
