# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""

Script to allocate a specific user to a node.

This script allocates the specified user to a node.  A particular node
may be specified, or the best available node used by default.

The allocated node is printed to stdout.

"""

import logging
import optparse

from database import Database
import util


logger = logging.getLogger("tokenserver.scripts.allocate_user")


def allocate_user(email, node=None):
    logger.info("Allocating node for user %s", email)
    try:
        database = Database()
        user = database.get_user(email)
        if user is None:
            user = database.allocate_user(email, node=node)
        else:
            database.update_user(user, node=node)
    except Exception:
        logger.exception("Error while updating node")
        return False
    else:
        logger.info("Finished updating node %s", node)
        return True


def main(args=None):
    """Main entry-point for running this script.

    This function parses command-line arguments and passes them on
    to the allocate_user() function.
    """
    usage = "usage: %prog [options] email [node_name]"
    descr = (
        "Allocate a user to a node.  You may specify a particular node, "
        "or omit to use the best available node."
    )
    parser = optparse.OptionParser(usage=usage, description=descr)
    parser.add_option(
        "-v",
        "--verbose",
        action="count",
        dest="verbosity",
        help="Control verbosity of log messages",
    )

    opts, args = parser.parse_args(args)
    if not 1 <= len(args) <= 2:
        parser.print_usage()
        return 1

    util.configure_script_logging(opts)

    email = args[0]
    if len(args) == 1:
        node_name = None
    else:
        node_name = args[1]

    allocate_user(email, node_name)
    return 0


if __name__ == "__main__":
    util.run_script(main)
