# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""

Script to process account-related events from an SQS queue.

This script polls an SQS queue for events indicating activity on an upstream
account, as documented here:

  https://github.com/mozilla/fxa-auth-server/blob/master/docs/service_notifications.md

The following event types are currently supported:

  * "delete":  the account was deleted; we mark their records as retired
               so they'll be cleaned up by our garbage-collection process.

  * "reset":   the account password was reset; we update our copy of their
               generation number to disconnect other devices.

  * "passwordChange":  the account password was changed; we update our copy
                       of their generation number to disconnect other devices.

Note that this is a purely optional administrative task, highly specific to
Mozilla's internal Firefox-Accounts-supported deployment.

"""

import json
import logging
import optparse

import boto
import boto.ec2
import boto.sqs
import boto.sqs.message
import boto.utils

import util
from database import Database


# Logging is initialized in `main` by `util.configure_script_logging()`
# Please do not call `logging.basicConfig()` before then, since this may
# cause duplicate error messages to be generated.
APP_LABEL = "tokenserver.scripts.process_account_events"


def process_account_events(
    queue_name, aws_region=None, queue_wait_time=20, metrics=None
):
    """Process account events from an SQS queue.

    This function polls the specified SQS queue for account-realted events,
    processing each as it is found.  It polls indefinitely and does not return;
    to interrupt execution you'll need to e.g. SIGINT the process.
    """
    logger = logging.getLogger(APP_LABEL)
    logger.info(f"Processing account events from {queue_name}")
    database = Database()
    try:
        # Connect to the SQS queue.
        # If no region is given, infer it from the instance metadata.
        if aws_region is None:
            logger.debug("Finding default region from instance metadata")
            aws_info = boto.utils.get_instance_metadata()
            aws_region = aws_info["placement"]["availability-zone"][:-1]
        logger.debug("Connecting to queue %r in %r", queue_name, aws_region)
        conn = boto.sqs.connect_to_region(aws_region)
        queue = conn.get_queue(queue_name)
        # We must force boto not to b64-decode the message contents, ugh.
        queue.set_message_class(boto.sqs.message.RawMessage)
        # Poll for messages indefinitely.
        while True:
            msg = queue.read(wait_time_seconds=queue_wait_time)
            if msg is None:
                continue
            process_account_event(database, msg.get_body(), metrics=metrics)
            # This intentionally deletes the event even if it was some
            # unrecognized type.  Not point leaving a backlog.
            queue.delete_message(msg)
    except Exception:
        logger.exception("Error while processing account events")
        raise


def process_account_event(database, body, metrics=None):
    """Parse and process a single account event."""
    # Try very hard not to error out if there's junk in the queue.
    logger = logging.getLogger(APP_LABEL)
    email = None
    event_type = None
    generation = None
    try:
        body = json.loads(body)
        event = json.loads(body["Message"])
        event_type = event["event"]
        uid = event["uid"]
        # Older versions of the fxa-auth-server would send an email-like
        # identifier the "uid" field, but that doesn't make sense for any
        # relier other than tokenserver.  Newer versions send just the raw uid
        # in the "uid" field, and include the domain in a separate "iss" field.
        if "iss" in event:
            email = f"{uid}@{event['iss']}"
        else:
            if "@" not in uid:
                raise ValueError("uid field does not contain issuer info")
            email = uid
        if event_type in (
            "reset",
            "passwordChange",
        ):
            generation = event["generation"]
    except (ValueError, KeyError) as e:
        logger.exception(f"Invalid account message: {e}", )
    else:
        if email is not None:
            record_metric = True
            if event_type == "delete":
                # Mark the user as retired.
                # Actual cleanup is done by a separate process.
                logger.info("Processing account delete for %r", email)
                database.retire_user(email)
            elif event_type == "reset":
                logger.info("Processing account reset for %r", email)
                update_generation_number(database, email, generation, metrics=metrics)
            elif event_type == "passwordChange":
                logger.info("Processing password change for %r", email)
                update_generation_number(database, email, generation, metrics=metrics)
            else:
                record_metric = False
                logger.warning("Dropping unknown event type %r", event_type)
            if record_metric and metrics:
                metrics.incr(event_type)


def update_generation_number(database, email, generation, metrics=None):
    """Update the maximum recorded generation number for the given user.

    When the FxA server sends us an update to the user's generation
    number, we want to update our high-water-mark in the DB in order to
    immediately lock out disconnected devices.  However, since we don't
    know the new value of the client state that goes with it, we can't just
    record the new generation number in the DB.  If we did, the first
    device that tried to sync with the new generation number would appear
    to have an incorrect client state value, and would be rejected.

    Instead, we take advantage of the fact that it's a timestamp, and write
    it into the DB at one millisecond less than its current value.  This
    ensures that we lock out any devices with an older generation number
    while avoiding errors with client state handling.

    This does leave a tiny edge-case where we can fail to lock out older
    devices, if the generation number changes twice in less than a
    millisecond.  This is acceptably unlikely in practice, and we'll recover
    as soon as we see an updated generation number as part of a sync.
    """
    user = database.get_user(email)
    if user is not None:
        database.update_user(user, generation - 1)
        if metrics:
            metrics.incr("decr_generation")


def main(args=None):
    """Main entry-point for running this script.

    This function parses command-line arguments and passes them on
    to the process_account_events() function.
    """
    usage = "usage: %prog [options] queue_name"
    parser = optparse.OptionParser(usage=usage)
    parser.add_option(
        "", "--aws-region", help="aws region in which the queue can be found"
    )
    parser.add_option(
        "",
        "--queue-wait-time",
        type="int",
        default=20,
        help="Number of seconds to wait for jobs on the queue",
    )
    parser.add_option(
        "-v",
        "--verbose",
        action="count",
        dest="verbosity",
        help="Control verbosity of log messages",
    )
    parser.add_option(
        "", "--human_logs", action="store_true", help="Human readable logs"
    )
    util.add_metric_options(parser)

    opts, args = parser.parse_args(args)
    # set up logging
    logger = util.configure_script_logging(opts, logger_name=APP_LABEL)

    logger.info("Starting up..")

    # set up metrics:
    metrics = util.Metrics(opts, namespace="tokenserver")

    if len(args) != 1:
        parser.print_usage()
        return 1

    queue_name = args[0]

    process_account_events(
        queue_name, opts.aws_region, opts.queue_wait_time, metrics=metrics
    )
    return 0


if __name__ == "__main__":
    util.run_script(main)
