# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""

Script to purge user records that have been replaced.

This script takes a tokenserver config file, uses it to load the assignment
backend, and then purges any obsolete user records from that backend.
Obsolete records are those that have been replaced by a newer record for
the same user.

Note that this is a purely optional administrative task, since replaced records
are handled internally by the assignment backend.  But it should help reduce
overheads, improve performance etc if run regularly.

"""

import os
import time
import random
import logging
import optparse

import requests
import tokenlib
import hawkauthlib

import tokenserver.scripts
from tokenserver.assignment import INodeAssignment
from tokenserver.util import format_key_id


logger = logging.getLogger("tokenserver.scripts.purge_old_records")


def purge_old_records(config_file, grace_period=-1, max_per_loop=10,
                      max_offset=0, request_timeout=60, settings=None):
    """Purge old records from the assignment backend in the given config file.

    This function iterates through each storage backend in the given config
    file and calls its purge_expired_items() method.  The result is a
    gradual pruning of expired items from each database.

    `max_offset` is used to select a random offset into the list of purgeable
    records. With multiple tasks running concurrently, this will provide each
    a (likely) different set of records to work on. A cheap, imperfect
    randomization.
    """
    logger.info("Purging old user records")
    logger.debug("Using config file %r", config_file)
    config = tokenserver.scripts.load_configurator(config_file)
    config.begin()
    try:
        backend = config.registry.getUtility(INodeAssignment)
        patterns = config.registry['endpoints_patterns']
        for service in patterns:
            previous_list = []
            logger.debug("Purging old user records for service: %s", service)
            # Process batches of <max_per_loop> items, until we run out.
            while True:
                offset = random.randint(0, max_offset)
                kwds = {
                    "grace_period": grace_period,
                    "limit": max_per_loop,
                    "offset": offset,
                }
                rows = list(backend.get_old_user_records(service, **kwds))
                if not rows:
                    logger.info("No more data for %s", service)
                    break
                if rows == previous_list:
                    raise Exception("Loop detected")
                previous_list = rows
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
                            backend.delete_user_record(service, row.uid)
                    elif not row.downed:
                        logger.info("Purging uid %s on %s", row.uid, row.node)
                        if settings and not settings.dryrun:
                            delete_service_data(config, service, row,
                                                timeout=request_timeout,
                                                settings=settings)
                            backend.delete_user_record(service, row.uid)
                        counter += 1
                    elif settings and settings.force:
                        logger.info(
                            "Forcing tokenserver record delete: {}".format(
                                row.uid
                            )
                        )
                        logger.info("Forcing Purge uid %s on %s",
                                    row.uid,
                                    row.node)
                        if not settings.dryrun:
                            delete_service_data(config, service, row,
                                                timeout=request_timeout,
                                                settings=settings)
                            backend.delete_user_record(service, row.uid)
                        counter += 1
                    if settings and settings.max_records:
                        if counter >= settings.max_records:
                            logger.info("Reached max_records, exiting")
                            return True
                if len(rows) < max_per_loop:
                    break
    except Exception as e:
        logger.exception("Error while purging old user records: {}".format(e))
        return False
    else:
        logger.info("Finished purging old user records")
        return True
    finally:
        config.end()


def delete_service_data(config, service, user, timeout=60, settings=None):
    """Send a data-deletion request to the user's service node.

    This is a little bit of hackery to cause the user's service node to
    remove any data it still has stored for the user.  We simulate a DELETE
    request from the user's own account.
    """
    secrets = config.registry.settings['tokenserver.secrets']
    pattern = config.registry['endpoints_patterns'][service]
    node_secrets = secrets.get(user.node)
    if not node_secrets:
        msg = "The node %r does not have any shared secret" % (user.node,)
        raise ValueError(msg)
    token = tokenlib.make_token({
        "uid": user.uid,
        "node": user.node,
        "fxa_uid": user.email.split("@", 1)[0],
        "fxa_kid": format_key_id(
            user.keys_changed_at or user.generation,
            user.client_state.decode('hex')
        ),
    }, secret=node_secrets[-1])
    secret = tokenlib.get_derived_secret(token, secret=node_secrets[-1])
    endpoint = pattern.format(uid=user.uid, service=service, node=user.node)
    auth = HawkAuth(token, secret)
    if settings and settings.dryrun:
        return
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
    usage = "usage: %prog [options] config_file"
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
    parser.add_option("", "--max-records", type="int", default=0,
                      help="Max records to delete")
    parser.add_option("", "--dryrun", action="store_true",
                      help="Don't do destructive things")
    parser.add_option("", "--force", action="store_true",
                      help="force record to be deleted from TS db,"
                      " even if node is down")
    parser.add_option("-v", "--verbose", action="count", dest="verbosity",
                      help="Control verbosity of log messages")

    opts, args = parser.parse_args(args)
    if len(args) != 1:
        parser.print_usage()
        return 1

    tokenserver.scripts.configure_script_logging(opts)

    config_file = os.path.abspath(args[0])

    purge_old_records(config_file,
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
            purge_old_records(config_file,
                              grace_period=opts.grace_period,
                              max_per_loop=opts.max_per_loop,
                              max_offset=opts.max_offset,
                              request_timeout=opts.request_timeout,
                              settings=opts)
    return 0


if __name__ == "__main__":
    tokenserver.scripts.run_script(main)
