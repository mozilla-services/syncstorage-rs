# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""

Script to purge user records that have been replaced.

This script purges any obsolete user records from the database.
Obsolete records are those that have been replaced by a newer record for
the same user.

Note that this is a purely optional administrative task, since replaced records
are handled internally by the assignment backend.  But it should help reduce
overheads, improve performance etc if run regularly.

"""

import binascii
import hawkauthlib
import logging
import optparse
import random
import requests
import time
import tokenlib

import util
from database import Database
from util import format_key_id


logger = logging.getLogger("tokenserver.scripts.purge_old_records")

PATTERN = "{node}/1.5/{uid}"


def purge_old_records(secret, grace_period=-1, max_per_loop=10, max_offset=0,
                      request_timeout=60, settings=None):
    """Purge old records from the database.

    This function queries all of the old user records in the database, deletes
    the Tokenserver database record for each of the users, and issues a delete
    request to each user's storage node. The result is a gradual pruning of
    expired items from each database.

    `max_offset` is used to select a random offset into the list of purgeable
    records. With multiple tasks running concurrently, this will provide each
    a (likely) different set of records to work on. A cheap, imperfect
    randomization.
    """
    logger.info("Purging old user records")
    try:
        database = Database()
        previous_list = []
        # Process batches of <max_per_loop> items, until we run out.
        while True:
            offset = random.randint(0, max_offset)
            kwds = {
                "grace_period": grace_period,
                "limit": max_per_loop,
                "offset": offset,
            }
            rows = list(database.get_old_user_records(**kwds))
            if not rows:
                logger.info("No more data")
                break
            if rows == previous_list:
                raise Exception("Loop detected")
            logger.info("Fetched %d rows at offset %d", len(rows), offset)
            counter = 0
            for row in rows:
                # Don't attempt to purge data from downed nodes.
                # Instead wait for them to either come back up or to be
                # completely removed from service.
                if row.node is None:
                    logger.info("Deleting user record for uid %s on %s",
                                row.uid, row.node)
                    if settings and not settings.dryrun:
                        database.delete_user_record(row.uid)
                elif not row.downed:
                    logger.info("Purging uid %s on %s", row.uid, row.node)
                    if settings and not settings.dryrun:
                        delete_service_data(row, secret, timeout=request_timeout, settings=settings)
                        database.delete_user_record(row.uid)
                    counter += 1
                elif settings and settings.force:
                    logger.info(
                        f"Forcing tokenserver record delete: {row.uid} on {row.node}"
                    )
                    if not settings.dryrun:
                        delete_service_data(row, secret, timeout=request_timeout, settings=settings)
                        database.delete_user_record(row.uid)
                    counter += 1
                if settings and settings.max_records:
                    if counter >= settings.max_records:
                        logger.info("Reached max_records, exiting")
                        return True
            if len(rows) < max_per_loop:
                break
    except Exception:
        logger.exception("Error while purging old user records")
        return False
    else:
        logger.info("Finished purging old user records")
        return True


def delete_service_data(user, secret, timeout=60, settings=None):
    """Send a data-deletion request to the user's service node.

    This is a little bit of hackery to cause the user's service node to
    remove any data it still has stored for the user.  We simulate a DELETE
    request from the user's own account.
    """
    token = tokenlib.make_token({
        "uid": user.uid,
        "node": user.node,
        "fxa_uid": user.email.split("@", 1)[0],
        "fxa_kid": format_key_id(
            user.keys_changed_at or user.generation,
            binascii.unhexlify(user.client_state)
        ),
    }, secret=secret)
    secret = tokenlib.get_derived_secret(token, secret=secret)
    endpoint = PATTERN.format(uid=user.uid, node=user.node)
    auth = HawkAuth(token, secret)
    resp = requests.delete(endpoint, auth=auth, timeout=timeout)
    if resp.status_code >= 400 and resp.status_code != 404:
        resp.raise_for_status()


class HawkAuth(requests.auth.AuthBase):
    """Hawk-signing auth helper class."""

    def __init__(self, token, secret):
        self.token = token
        self.secret = secret

    def __call__(self, req):
        hawkauthlib.sign_request(req, self.token, self.secret)
        return req


def main(args=None):
    """Main entry-point for running this script.

    This function parses command-line arguments and passes them on
    to the purge_old_records() function.
    """
    usage = "usage: %prog [options] secret"
    parser = optparse.OptionParser(usage=usage)
    parser.add_option("", "--purge-interval", type="int", default=3600,
                      help="Interval to sleep between purging runs")
    parser.add_option("", "--grace-period", type="int", default=86400,
                      help="Number of seconds grace to allow on replacement")
    parser.add_option("", "--max-per-loop", type="int", default=10,
                      help="Maximum number of items to fetch in one go")
    # N.B., if the number of purgeable rows is <<< max_offset then most
    # selects will return zero rows. Choose this value accordingly.
    parser.add_option("", "--max-offset", type="int", default=0,
                      help="Use random offset from 0 to max_offset")
    parser.add_option("", "--request-timeout", type="int", default=60,
                      help="Timeout for service deletion requests")
    parser.add_option("", "--oneshot", action="store_true",
                      help="Do a single purge run and then exit")
    parser.add_option("-v", "--verbose", action="count", dest="verbosity",
                      help="Control verbosity of log messages")
    parser.add_option("", "--dryrun", action="store_true",
                      help="Don't do destructive things")
    parser.add_option("", "--force", action="store_true",
                      help="force record to be deleted from TS db,"
                      " even if node is down")

    opts, args = parser.parse_args(args)
    if len(args) != 2:
        parser.print_usage()
        return 1

    secret = args[1]

    util.configure_script_logging(opts)

    purge_old_records(secret,
                      grace_period=opts.grace_period,
                      max_per_loop=opts.max_per_loop,
                      max_offset=opts.max_offset,
                      request_timeout=opts.request_timeout,
                      settings=opts)
    if not opts.oneshot:
        while True:
            # Randomize sleep interval +/- thirty percent to desynchronize
            # instances of this script running on multiple webheads.
            sleep_time = opts.purge_interval
            sleep_time += random.randint(-0.3 * sleep_time, 0.3 * sleep_time)
            logger.debug("Sleeping for %d seconds", sleep_time)
            time.sleep(sleep_time)
            purge_old_records(grace_period=opts.grace_period,
                              max_per_loop=opts.max_per_loop,
                              max_offset=opts.max_offset,
                              request_timeout=opts.request_timeout,
                              settings=opts)
    return 0


if __name__ == "__main__":
    util.run_script(main)
