# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""

Script to emit total-user-count metrics for exec dashboard.

"""

import json
import logging
import optparse
import os
import socket
import sys
import time
from datetime import datetime, timedelta, tzinfo

from database import Database
import util

logger = logging.getLogger("tokenserver.scripts.count_users")

ZERO = timedelta(0)


class UTC(tzinfo):
    def utcoffset(self, dt):
        return ZERO

    def tzname(self, dt):
        return "UTC"

    def dst(self, dt):
        return ZERO


utc = UTC()


def count_users(outfile, timestamp=None):
    if timestamp is None:
        ts = time.gmtime()
        midnight = (ts[0], ts[1], ts[2], 0, 0, 0, ts[6], ts[7], ts[8])
        timestamp = int(time.mktime(midnight)) * 1000
    database = Database()
    logger.debug("Counting users created before %i", timestamp)
    count = database.count_users(timestamp)
    logger.debug("Found %d users", count)
    # Output has heka-filter-compatible JSON object.
    ts_sec = timestamp / 1000
    output = {
        "hostname": socket.gethostname(),
        "pid": os.getpid(),
        "op": "sync_count_users",
        "total_users": count,
        "time": datetime.fromtimestamp(ts_sec, utc).isoformat(),
        "v": 0,
    }
    json.dump(output, outfile)
    outfile.write("\n")


def main(args=None):
    """Main entry-point for running this script.

    This function parses command-line arguments and passes them on
    to the add_node() function.
    """
    usage = "usage: %prog [options]"
    descr = "Count total users in the tokenserver database"
    parser = optparse.OptionParser(usage=usage, description=descr)
    parser.add_option(
        "-t",
        "--timestamp",
        type="int",
        help="Max creation timestamp; default previous midnight",
    )
    parser.add_option("-o", "--output", help="Output file; default stderr")
    parser.add_option(
        "-v",
        "--verbose",
        action="count",
        dest="verbosity",
        help="Control verbosity of log messages",
    )

    opts, args = parser.parse_args(args)
    if len(args) != 0:
        parser.print_usage()
        return 1

    util.configure_script_logging(opts)

    if opts.output in (None, "-"):
        count_users(sys.stdout, opts.timestamp)
    else:
        with open(opts.output, "a") as outfile:
            count_users(outfile, opts.timestamp)

    return 0


if __name__ == "__main__":
    util.run_script(main)
