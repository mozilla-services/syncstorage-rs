# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""

Script to update node status in the db.

"""

import logging
import optparse

from database import Database
import util


logger = logging.getLogger("tokenserver.scripts.update_node")


def update_node(node, **kwds):
    """Update details of a node."""
    logger.info("Updating node %s for service %s", node)
    logger.debug("Value: %r", kwds)
    try:
        database = Database()
        database.update_node(node, **kwds)
    except Exception:
        logger.exception("Error while updating node")
        return False
    else:
        logger.info("Finished updating node %s", node)
        return True


def main(args=None):
    """Main entry-point for running this script.

    This function parses command-line arguments and passes them on
    to the update_node() function.
    """
    usage = "usage: %prog [options] node_name"
    descr = "Update node details in the tokenserver database"
    parser = optparse.OptionParser(usage=usage, description=descr)
    parser.add_option(
        "", "--capacity", type="int", help="How many user slots the node has overall"
    )
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
    if len(args) != 1:
        parser.print_usage()
        return 1

    util.configure_script_logging(opts)

    node_name = args[0]

    kwds = {}
    if opts.capacity is not None:
        kwds["capacity"] = opts.capacity
    if opts.available is not None:
        kwds["available"] = opts.available
    if opts.current_load is not None:
        kwds["current_load"] = opts.current_load
    if opts.backoff is not None:
        kwds["backoff"] = opts.backoff
    if opts.downed is not None:
        kwds["downed"] = opts.downed

    update_node(node_name, **kwds)
    return 0


if __name__ == "__main__":
    util.run_script(main)
