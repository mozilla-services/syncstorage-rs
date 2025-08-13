#! /usr/bin/env python
# script to populate the database with records
import time
import random
from sqlalchemy import create_engine
from sqlalchemy.sql import text as sqltext

_CREATE_USER_RECORD = sqltext("""\
insert into
    users
    (service, email, nodeid, generation, client_state,
     created_at, replaced_at)
values
    (:service, :email, :nodeid, 0, "", :timestamp, NULL)
""")

_GET_SERVICE_ID = sqltext("""\
select
    id
from
    services
where
    service = :service
""")

_GET_NODE_ID = sqltext("""\
select
    id
from
    nodes
where
    service=:service and node=:node
""")

_SERVICE_NAME = "sync-1.5"


# This class creates a bunch of users associated with the sync-1.5 service.
#
# The resulting users will have an address in the form of <uid>@<host> where
# uid is an int from 0 to :param user_range:.
#
# This class is useful to populate the database during the load tests. It
# allows us to test a specific behaviour: making sure that we are not reading
# the values from memory when retrieving the node information.
#
# :param sqluri: the sqluri string used to connect to the database
# :param nodes: the list of available nodes for this service
# :param user_range: the number of users to create
# :param host: the hostname to use when generating users
class PopulateDatabase:
    def __init__(self, sqluri, nodes, user_range, host="loadtest.local"):
        engine = create_engine(sqluri)
        self.database = engine.execution_options(isolation_level="AUTOCOMMIT").connect()

        self.service_id = self._get_service_id()
        self.node_ids = [self._get_node_id(node) for node in nodes]
        self.user_range = user_range
        self.host = host

    def _get_node_id(self, node_name):
        """Get numeric id for a node."""
        res = self.database.execute(
            _GET_NODE_ID, service=self.service_id, node=node_name
        )
        row = res.fetchone()
        res.close()
        if row is None:
            raise ValueError("unknown node: " + node_name)
        return row[0]

    def _get_service_id(self):
        res = self.database.execute(_GET_SERVICE_ID, service=_SERVICE_NAME)
        row = res.fetchone()
        res.close()
        return row.id

    def run(self):
        params = {
            "service": self.service_id,
            "timestamp": int(time.time() * 1000),
        }

        # for each user in the range, assign them to a node
        for idx in range(0, self.user_range):
            email = "%s@%s" % (idx, self.host)
            nodeid = random.choice(self.node_ids)
            self.database.execute(
                _CREATE_USER_RECORD, email=email, nodeid=nodeid, **params
            )


def main():
    # Read the arguments from the command line and pass them to the
    # PopulateDb class.
    #
    # Example use:
    #
    #     python3 populate-db.py sqlite:////tmp/tokenserver\
    #     node1,node2,node3,node4,node5,node6 100
    import sys

    if len(sys.argv) < 4:
        raise ValueError(
            "You need to specify (in this order) sqluri, "
            "nodes (comma separated), and user_range"
        )
    # transform the values from the cli to python objects
    sys.argv[2] = sys.argv[2].split(",")  # comma separated => list
    sys.argv[3] = int(sys.argv[3])

    PopulateDatabase(*sys.argv[1:]).run()
    print("created {nb_users} users".format(nb_users=sys.argv[3]))


if __name__ == "__main__":
    main()
