# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""

Script to remove a node from the system.

This script clears any assignments to the named node.

"""

import logging
import optparse

from database import Database
import util


logger = logging.getLogger("tokenserver.scripts.unassign_node")


def unassign_node(node):
    """Clear any assignments to the named node."""
    logger.info("Unassignment node %s", node)
    try:
        database = Database()
        found = False
        try:
            database.unassign_node(node)
        except ValueError:
            logger.debug("  not found")
        else:
            found = True
            logger.debug("  unassigned")
    except Exception:
        logger.exception("Error while unassigning node")
        return False
    else:
        if not found:
            logger.info("Node %s was not found", node)
        else:
            logger.info("Finished unassigning node %s", node)
        return True


def main(args=None):
    """Main entry-point for running this script.

    This function parses command-line arguments and passes them on
    to the unassign_node() function.
    """
    usage = "usage: %prog [options] node_name"
    descr = "Clear all assignments to node in the tokenserver database"
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

    unassign_node(node_name)
    return 0


if __name__ == "__main__":
    util.run_script(main)
