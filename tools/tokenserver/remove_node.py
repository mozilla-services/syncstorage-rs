# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""

Script to remove a node from the system.

This script nukes any references to the named node - it is removed from
the "nodes" table and any users currently assigned to that node have their
assignments cleared.

"""

import logging
import optparse

import util
from database import Database

logger = logging.getLogger("tokenserver.scripts.remove_node")


def remove_node(node):
    """Remove the named node from the system."""
    logger.info("Removing node %s", node)
    try:
        database = Database()
        found = False
        try:
            database.remove_node(node)
        except ValueError:
            logger.debug("  not found")
        else:
            found = True
            logger.debug("  removed")
    except Exception:
        logger.exception("Error while removing node")
        return False
    else:
        if not found:
            logger.info("Node %s was not found", node)
        else:
            logger.info("Finished removing node %s", node)
        return True


def main(args=None):
    """Main entry-point for running this script.

    This function parses command-line arguments and passes them on
    to the remove_node() function.
    """
    usage = "usage: %prog [options] node_name"
    descr = "Remove a node from the tokenserver database"
    parser = optparse.OptionParser(usage=usage, description=descr)
    parser.add_option(
        "-v",
        "--verbose",
        action="count",
        dest="verbosity",
        help="Control verbosity of log messages",
    )

    opts, args = parser.parse_args(args)
    if len(args) != 1:
        parser.print_usage()
        return 1

    util.configure_script_logging(opts)

    node_name = args[0]

    remove_node(node_name)
    return 0


if __name__ == "__main__":
    util.run_script(main)
