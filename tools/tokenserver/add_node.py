# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.

"""

Script to add a new node to the system.

"""

import logging
import optparse

from database import Database, SERVICE_NAME
import util


logger = logging.getLogger("tokenserver.scripts.add_node")


def add_node(node, capacity, **kwds):
    """Add the specific node to the system."""
    logger.info("Adding node %s to service %s", node, SERVICE_NAME)
    try:
        database = Database()
        database.add_node(node, capacity, **kwds)
    except Exception:
        logger.exception("Error while adding node")
        return False
    else:
        logger.info("Finished adding node %s", node)
        return True


def main(args=None):
    """Main entry-point for running this script.

    This function parses command-line arguments and passes them on
    to the add_node() function.
    """
    usage = "usage: %prog [options] node_name capacity"
    descr = "Add a new node to the tokenserver database"
    parser = optparse.OptionParser(usage=usage, description=descr)
    parser.add_option(
        "", "--available", type="int", help="How many user slots the node has available"
    )
    parser.add_option(
        "",
        "--current-load",
        type="int",
        help="How many user slots the node has occupied",
    )
    parser.add_option(
        "", "--downed", action="store_true", help="Mark the node as down in the db"
    )
    parser.add_option(
        "",
        "--backoff",
        action="store_true",
        help="Mark the node as backed-off in the db",
    )
    parser.add_option(
        "-v",
        "--verbose",
        action="count",
        dest="verbosity",
        help="Control verbosity of log messages",
    )

    opts, args = parser.parse_args(args)
    if len(args) != 2:
        parser.print_usage()
        return 1

    util.configure_script_logging(opts)

    node_name = args[0]
    capacity = int(args[1])

    kwds = {}
    if opts.available is not None:
        kwds["available"] = opts.available
    if opts.current_load is not None:
        kwds["current_load"] = opts.current_load
    if opts.backoff is not None:
        kwds["backoff"] = opts.backoff
    if opts.downed is not None:
        kwds["downed"] = opts.downed

    add_node(node_name, capacity, **kwds)
    return 0


if __name__ == "__main__":
    util.run_script(main)
