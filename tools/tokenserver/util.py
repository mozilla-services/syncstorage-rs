# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""

Admin/managment scripts for TokenServer.

"""

import sys
import time
import logging

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
    if not opts or not opts.verbosity:
        loglevel = logging.WARNING
    elif opts.verbosity == 1:
        loglevel = logging.INFO
    else:
        loglevel = logging.DEBUG

    handler = logging.StreamHandler()
    handler.setFormatter(logging.Formatter("%(message)s"))
    handler.setLevel(loglevel)
    logger = logging.getLogger("")
    logger.addHandler(handler)
    logger.setLevel(loglevel)


def format_key_id(keys_changed_at, key_hash):
    """Format an FxA key ID from a timestamp and key hash."""
    return "{:013d}-{}".format(
        keys_changed_at,
        encode_bytes_b64(key_hash),
    )


def get_timestamp():
    """Get current timestamp in milliseconds."""
    return int(time.time() * 1000)
