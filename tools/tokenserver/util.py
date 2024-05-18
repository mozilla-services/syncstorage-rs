# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""

Admin/managment scripts for TokenServer.

"""

import sys
import time
import logging
import os
import json
from datetime import datetime

from datadog import initialize, statsd

from browserid.utils import encode_bytes as encode_bytes_b64


def run_script(main):
    """Simple wrapper for running scripts in __main__ section."""
    try:
        exitcode = main()
    except KeyboardInterrupt:
        exitcode = 1
    sys.exit(exitcode)


def configure_script_logging(opts=None):
    """Configure stdlib logging to produce output from the script.

    This basically configures logging to send messages to stderr, with
    formatting that's more for human readability than machine parsing.
    It also takes care of the --verbosity command-line option.
    """

    verbosity = (
        opts and getattr(
            opts, "verbosity", logging.NOTSET)) or logging.NOTSET
    logger = logging.getLogger(getattr(opts, "app_label", ""))
    level = os.environ.get("PYTHON_LOG", "").upper() or \
        max(logging.DEBUG, logging.WARNING - (verbosity * 10)) or \
        logger.getEffectiveLevel()

    # if we've already written a log message, the handler may already
    # be defined. We don't want to duplicate messages if possible, so
    # check and potentially adjust the existing logger's handler.
    if logger.hasHandlers():
        handler = logger.handlers[0]
    else:
        handler = logging.StreamHandler()

    formatter = GCP_JSON_Formatter()
    # if we've opted for "human_logs", specify a simpler message.
    if opts:
        if getattr(opts, "human_logs", None):
            formatter = logging.Formatter(
                "{levelname:<8s}: {message}",
                style="{")

    handler.setFormatter(formatter)
    handler.setLevel(level)
    logger = logging.getLogger("")
    logger.addHandler(handler)
    logger.setLevel(level)


# We need to reformat a few things to get the record to display correctly
# This includes "escaping" the message as well as converting the timestamp
# into a parsable format.
class GCP_JSON_Formatter(logging.Formatter):

    def format(self, record):
        return json.dumps({
            "severity": record.levelname,
            "message": record.getMessage(),
            "timestamp": datetime.fromtimestamp(
                record.created).strftime(
                    "%Y-%m-%dT%H:%M:%SZ"  # RFC3339
                ),
        })


def format_key_id(keys_changed_at, key_hash):
    """Format an FxA key ID from a timestamp and key hash."""
    return "{:013d}-{}".format(
        keys_changed_at,
        encode_bytes_b64(key_hash),
    )


def get_timestamp():
    """Get current timestamp in milliseconds."""
    return int(time.time() * 1000)


class Metrics():

    def __init__(self, opts):
        options = dict(
            namespace=getattr(opts, "app_label", ""),
            statsd_namespace=getattr(opts, "app_label", ""),
            statsd_host=getattr(
                opts, "metric_host", os.environ.get("METRIC_HOST")),
            statsd_port=getattr(
                opts, "metric_port", os.environ.get("METRIC_PORT")),
        )
        self.prefix = options.get("namespace")
        initialize(**options)

    def incr(self, label, tags=None):
        statsd.increment(label, tags=tags)
