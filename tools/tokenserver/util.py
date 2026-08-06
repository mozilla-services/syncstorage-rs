# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Admin/management scripts for TokenServer."""

import sys
import time
import logging
import base64
import os
import json
from datetime import datetime

# ``Metrics`` and ``add_metric_options`` now live in ``tools/common``; they
# are re-exported here because the tokenserver scripts import them from this
# module.  Resolution depends on the path: the deployment runs with
# ``PYTHONPATH=/app/tools/tokenserver`` (so ``common`` needs ``tools/``
# added), but pytest runs from the repo root (where the package is
# ``tools.common``). Try both.
try:
    from common.metrics import Metrics, add_metric_options
except ImportError:
    sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    from common.metrics import Metrics, add_metric_options

__all__ = [
    "Metrics",
    "add_metric_options",
    "configure_script_logging",
    "encode_bytes_b64",
    "format_key_id",
    "get_timestamp",
    "run_script",
    "GCP_JSON_Formatter",
]


def encode_bytes_b64(value):
    """Encode bytes to a URL-safe base64 string without padding."""
    return base64.urlsafe_b64encode(value).rstrip(b"=").decode("ascii")


def run_script(main):
    """Run a script's main function and exit with the returned code."""
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
    """JSON log formatter compatible with Google Cloud Platform logging."""

    def format(self, record):
        """Format a log record as a GCP-compatible JSON string."""
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
