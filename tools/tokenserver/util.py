# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""

Admin/managment scripts for TokenServer.

"""

import sys
import time
import logging
import base64
import optparse
import os
import json
from datetime import datetime

from datadog import initialize, statsd


def encode_bytes_b64(value):
    return base64.urlsafe_b64encode(value).rstrip(b"=").decode("ascii")


def run_script(main):
    """Simple wrapper for running scripts in __main__ section."""
    try:
        exitcode = main()
    except KeyboardInterrupt:
        exitcode = 1
    sys.exit(exitcode)


def configure_script_logging(opts=None, logger_name=""):
    """Configure stdlib logging to produce output from the script.

    This basically configures logging to send messages to stderr, with
    formatting that's more for human readability than machine parsing.
    It also takes care of the --verbosity command-line option.
    """

    verbosity = (opts and getattr(opts, "verbosity", logging.NOTSET)) or logging.NOTSET
    logger = logging.getLogger(logger_name)
    level = (
        os.environ.get("PYTHON_LOG", "").upper()
        or max(logging.DEBUG, logging.WARNING - (verbosity * 10))
        or logger.getEffectiveLevel()
    )

    # if we've previously setup a handler, adjust it instead
    if logger.hasHandlers():
        handler = logger.handlers[0]
    else:
        handler = logging.StreamHandler()

    formatter = GCP_JSON_Formatter()
    # if we've opted for "human_logs", specify a simpler message.
    if opts:
        if getattr(opts, "human_logs", None):
            formatter = logging.Formatter("{levelname:<8s}: {message}", style="{")

    handler.setFormatter(formatter)
    handler.setLevel(level)
    logger = logging.getLogger("")
    logger.addHandler(handler)
    logger.setLevel(level)
    return logger


# We need to reformat a few things to get the record to display correctly
# This includes "escaping" the message as well as converting the timestamp
# into a parsable format.
class GCP_JSON_Formatter(logging.Formatter):
    def format(self, record):
        return json.dumps(
            {
                "severity": record.levelname,
                "message": super().format(record),
                "timestamp": datetime.fromtimestamp(record.created).strftime(
                    "%Y-%m-%dT%H:%M:%SZ"  # RFC3339
                ),
            }
        )


def format_key_id(keys_changed_at, key_hash):
    """Format an FxA key ID from a timestamp and key hash."""
    return "{:013d}-{}".format(
        keys_changed_at,
        encode_bytes_b64(key_hash),
    )


def get_timestamp():
    """Get current timestamp in milliseconds."""
    return int(time.time() * 1000)


class Metrics:
    def __init__(self, opts, namespace=""):
        options = dict(
            namespace=namespace,
            statsd_namespace=namespace,
            statsd_host=getattr(opts, "metric_host"),
            statsd_port=getattr(opts, "metric_port"),
        )
        self.prefix = options.get("namespace")
        initialize(**options)

    def incr(self, label, tags=None):
        statsd.increment(label, tags=tags)


def add_metric_options(parser: optparse.OptionParser):
    """Add generic metric related options to an OptionParser"""
    parser.add_option(
        "",
        "--metric_host",
        default=os.environ.get("SYNC_STATSD_HOST"),
        help="Metric host name",
    )
    parser.add_option(
        "",
        "--metric_port",
        default=os.environ.get("SYNC_STATSD_PORT"),
        help="Metric host port",
    )
