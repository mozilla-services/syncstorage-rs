# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.

import math
import os
from urllib.parse import urlsplit

from sqlalchemy import create_engine
from sqlalchemy.sql import text as sqltext

from util import get_timestamp

# The maximum possible generation number.
# Used as a tombstone to mark users that have been "retired" from the db.
MAX_GENERATION = 9223372036854775807
NODE_FIELDS = ("capacity", "available", "current_load", "downed", "backoff")

_GET_USER_RECORDS = sqltext("""\
select
    uid, nodes.node, generation, keys_changed_at, client_state, created_at,
    replaced_at
from
    users left outer join nodes on users.nodeid = nodes.id
where
    email = :email and users.service = :service
order by
    created_at desc, uid desc
limit
    20
""")

_CREATE_USER_RECORD = sqltext("""\
insert into
    users
    (service, email, nodeid, generation, keys_changed_at, client_state,
     created_at, replaced_at)
values
    (:service, :email, :nodeid, :generation, :keys_changed_at,
     :client_state, :timestamp, NULL)
""")

# The `where` clause on this statement is designed as an extra layer of
# protection, to ensure that concurrent updates don't accidentally move
# timestamp fields backwards in time. The handling of `keys_changed_at`
# is additionally weird because we want to treat the default `NULL` value
# as zero.
_UPDATE_USER_RECORD_IN_PLACE = sqltext("""\
update
    users
set
    generation = COALESCE(:generation, generation),
    keys_changed_at = COALESCE(:keys_changed_at, keys_changed_at)
where
    service = :service and email = :email and
    generation <= COALESCE(:generation, generation) and
    COALESCE(keys_changed_at, 0) <=
        COALESCE(:keys_changed_at, keys_changed_at, 0) and
    replaced_at is null
""")


_REPLACE_USER_RECORDS = sqltext("""\
update
    users
set
    replaced_at = :timestamp
where
    service = :service and email = :email
    and replaced_at is null and created_at < :timestamp
""")


# Mark all records for the user as replaced,
# and set a large generation number to block future logins.
_RETIRE_USER_RECORDS = sqltext("""\
update
    users
set
    replaced_at = :timestamp,
    generation = :generation
where
    email = :email
    and replaced_at is null
""")


_GET_OLD_USER_RECORDS_FOR_SERVICE = sqltext("""\
select
    uid, email, generation, keys_changed_at, client_state,
    nodes.node, nodes.downed, created_at, replaced_at
from
    users left outer join nodes on users.nodeid = nodes.id
where
    users.service = :service
and
    replaced_at is not null and replaced_at < :timestamp
order by
    replaced_at desc, uid desc
limit
    :limit
offset
    :offset
""")

_GET_OLD_USER_RECORDS_FOR_SERVICE_RANGE = """\
select
    uid, email, generation, keys_changed_at, client_state,
    nodes.node, nodes.downed, created_at, replaced_at
from
    users left outer join nodes on users.nodeid = nodes.id
where
    users.service = :service
and
    ::RANGE::
and
    replaced_at is not null and replaced_at < :timestamp
order by
    replaced_at desc, uid desc
limit
    :limit
offset
    :offset
"""


_GET_ALL_USER_RECORDS_FOR_SERVICE = sqltext("""\
select
    uid, nodes.node, created_at, replaced_at
from
    users left outer join nodes on users.nodeid = nodes.id
where
    email = :email and users.service = :service
order by
    created_at asc, uid desc
""")


_REPLACE_USER_RECORD = sqltext("""\
update
    users
set
    replaced_at = :timestamp
where
    service = :service
and
    uid = :uid
""")


_DELETE_USER_RECORD = sqltext("""\
delete from
    users
where
    service = :service
and
    uid = :uid
""")


_FREE_SLOT_ON_NODE = sqltext("""\
update
    nodes
set
    available = available + 1, current_load = current_load - 1
where
    id = (SELECT nodeid FROM users WHERE service=:service AND uid=:uid)
""")


_COUNT_USER_RECORDS = sqltext("""\
select
    count(email)
from
    users
where
    replaced_at is null
    and created_at <= :timestamp
""")


_GET_BEST_NODE = sqltext("""\
select
    id, node
from
    nodes
where
    service = :service
    and available > 0
    and capacity > current_load
    and downed = 0
    and backoff = 0
order by
    log(current_load) / log(capacity)
limit 1
""")


_RELEASE_NODE_CAPACITY = sqltext("""\
update
    nodes
set
    available = least(capacity * :capacity_release_rate,
                      capacity - current_load)
where
    service = :service
    and available <= 0
    and capacity > current_load
    and downed = 0
""")


_ADD_USER_TO_NODE = sqltext("""\
update
    nodes
set
    current_load = current_load + 1,
    available = greatest(available - 1, 0)
where
    service = :service
    and node = :node
""")


_GET_SERVICE_ID = sqltext("""\
select
    id
from
    services
where
    service = :service
""")


_GET_NODE = sqltext("""\
select
    *
from
    nodes
where
    service = :service
    and node = :node
 """)


_GET_SPANNER_NODE = sqltext("""\
select
     id, node
from
     nodes
where
     id = :id
limit
    1
""")

SERVICE_NAME = "sync-1.5"


class Database:
    def __init__(self):
        # Formatted for rust diesel with a "postgres" dialect, whereas
        # sqlalchemy uses "postgresql" instead
        s = urlsplit(os.environ["SYNC_TOKENSERVER__DATABASE_URL"])
        if s.scheme == "postgres":
            s = s._replace(scheme="postgresql")
        self.db_mode = s.scheme
        engine = create_engine(s.geturl())
        self.database = engine.execution_options(isolation_level="AUTOCOMMIT").connect()
        self.capacity_release_rate = os.environ.get("NODE_CAPACITY_RELEASE_RATE", 0.1)
        self.spanner_node_id = os.environ.get("SYNC_TOKENSERVER__SPANNER_NODE_ID")
        self.spanner_node = None
        if self.spanner_node_id:
            self.spanner_node = self.get_spanner_node(self.spanner_node_id)

    def _execute_sql(self, *args, **kwds):
        return self.database.execute(*args, **kwds)

    def close(self):
        self.database.close()

    def get_user(self, email):
        params = {"service": self._get_service_id(SERVICE_NAME), "email": email}
        res = self._execute_sql(_GET_USER_RECORDS, **params)
        try:
            # The query fetches rows ordered by created_at, but we want
            # to ensure that they're ordered by (generation, created_at).
            # This is almost always true, except for strange race conditions
            # during row creation.  Sorting them is an easy way to enforce
            # this without bloating the db index.
            rows = res.fetchall()
            rows.sort(key=lambda r: (r.generation, r.created_at), reverse=True)
            if not rows:
                return None
            # The first row is the most up-to-date user record.
            # The rest give previously-seen client-state values.
            cur_row = rows[0]
            old_rows = rows[1:]
            user = {
                "email": email,
                "uid": cur_row.uid,
                "node": cur_row.node,
                "generation": cur_row.generation,
                "keys_changed_at": cur_row.keys_changed_at or 0,
                "client_state": cur_row.client_state,
                "old_client_states": {},
                "first_seen_at": cur_row.created_at,
            }
            # If the current row is marked as replaced or is missing a node,
            # and they haven't been retired, then assign them a new node.
            if cur_row.replaced_at is not None or cur_row.node is None:
                if cur_row.generation < MAX_GENERATION:
                    user = self.allocate_user(
                        email,
                        cur_row.generation,
                        cur_row.client_state,
                        cur_row.keys_changed_at,
                    )
            for old_row in old_rows:
                # Collect any previously-seen client-state values.
                if old_row.client_state != user["client_state"]:
                    user["old_client_states"][old_row.client_state] = True
                # Make sure each old row is marked as replaced.
                # They might not be, due to races in row creation.
                if old_row.replaced_at is None:
                    timestamp = cur_row.created_at
                    self.replace_user_record(old_row.uid, timestamp)
                # Track backwards to the oldest timestamp at which we saw them.
                user["first_seen_at"] = old_row.created_at
            return user
        finally:
            res.close()

    def allocate_user(
        self,
        email,
        generation=0,
        client_state="",
        keys_changed_at=0,
        node=None,
        timestamp=None,
    ):
        if timestamp is None:
            timestamp = get_timestamp()
        if node is None:
            nodeid, node = self.get_best_node()
        else:
            nodeid = self.get_node_id(node)
        params = {
            "service": self._get_service_id(SERVICE_NAME),
            "email": email,
            "nodeid": nodeid,
            "generation": generation,
            "keys_changed_at": keys_changed_at,
            "client_state": client_state,
            "timestamp": timestamp,
        }
        if self.db_mode == "postgresql":
            insert_sql = sqltext("""\
                insert into users (service, email, nodeid, generation, keys_changed_at, client_state, created_at, replaced_at)
                values (:service, :email, :nodeid, :generation, :keys_changed_at, :client_state, :timestamp, NULL)
                RETURNING uid
            """)
        else:
            insert_sql = _CREATE_USER_RECORD
        res = self._execute_sql(insert_sql, **params)

        if self.db_mode == "postgresql":
            uid = res.fetchone()[0]
        else:
            uid = res.lastrowid
        res.close()
        return {
            "email": email,
            "uid": uid,
            "node": node,
            "generation": generation,
            "keys_changed_at": keys_changed_at,
            "client_state": client_state,
            "old_client_states": {},
            "first_seen_at": timestamp,
        }

    def update_user(
        self, user, generation=None, client_state=None, keys_changed_at=None, node=None
    ):
        if client_state is None and node is None:
            # No need for a node-reassignment, just update the row in place.
            # Note that if we're changing keys_changed_at without changing
            # client_state, it's because we're seeing an existing value of
            # keys_changed_at for the first time.
            params = {
                "service": self._get_service_id(SERVICE_NAME),
                "email": user["email"],
                "generation": generation,
                "keys_changed_at": keys_changed_at,
            }
            res = self._execute_sql(_UPDATE_USER_RECORD_IN_PLACE, **params)
            res.close()

            if generation is not None:
                user["generation"] = max(user["generation"], generation)
            user["keys_changed_at"] = max_keys_changed_at(user, keys_changed_at)
        else:
            # Reject previously-seen client-state strings.
            if client_state is None:
                client_state = user["client_state"]
            else:
                if client_state == user["client_state"]:
                    raise Exception("previously seen client-state string")
                if client_state in user["old_client_states"]:
                    raise Exception("previously seen client-state string")
            # Need to create a new record for new user state.
            # If the node is not explicitly changing, try to keep them on the
            # same node, but if e.g. it no longer exists them allocate them to
            # a new one.
            if node is not None:
                nodeid = self.get_node_id(node)
                user["node"] = node
            else:
                try:
                    nodeid = self.get_node_id(user["node"])
                except ValueError:
                    nodeid, node = self.get_best_node()
                    user["node"] = node
            if generation is not None:
                generation = max(user["generation"], generation)
            else:
                generation = user["generation"]
            keys_changed_at = max_keys_changed_at(user, keys_changed_at)
            now = get_timestamp()
            params = {
                "service": self._get_service_id(SERVICE_NAME),
                "email": user["email"],
                "nodeid": nodeid,
                "generation": generation,
                "keys_changed_at": keys_changed_at,
                "client_state": client_state,
                "timestamp": now,
            }
            if self.db_mode == "postgresql":
                insert_sql = sqltext("""\
                insert into users (service, email, nodeid, generation, keys_changed_at, client_state, created_at, replaced_at)
                values (:service, :email, :nodeid, :generation, :keys_changed_at, :client_state, :timestamp, NULL)
                RETURNING uid
                """)
            else:
                insert_sql = _CREATE_USER_RECORD
            res = self._execute_sql(insert_sql, **params)

            if self.db_mode == "postgresql":
                uid = res.fetchone()[0]
            else:
                uid = res.lastrowid
            res.close()
            user["uid"] = uid
            user["generation"] = generation
            user["keys_changed_at"] = keys_changed_at
            user["old_client_states"][user["client_state"]] = True
            user["client_state"] = client_state
            # mark old records as having been replaced.
            # if we crash here, they are unmarked and we may fail to
            # garbage collect them for a while, but the active state
            # will be undamaged.
            self.replace_user_records(user["email"], now)

    def retire_user(self, email):
        now = get_timestamp()
        params = {"email": email, "timestamp": now, "generation": MAX_GENERATION}
        # Pass through explicit engine to help with sharded implementation,
        # since we can't shard by service name here.
        res = self._execute_sql(_RETIRE_USER_RECORDS, **params)
        res.close()

    def count_users(self, timestamp=None):
        if timestamp is None:
            timestamp = get_timestamp()
        res = self._execute_sql(_COUNT_USER_RECORDS, timestamp=timestamp)
        row = res.fetchone()
        res.close()
        return row[0]

    #
    # Methods for low-level user record management.
    #

    def get_user_records(self, email):
        """Get all the user's records, including the old ones."""
        params = {"service": self._get_service_id(SERVICE_NAME), "email": email}
        res = self._execute_sql(_GET_ALL_USER_RECORDS_FOR_SERVICE, **params)
        try:
            for row in res:
                yield row
        finally:
            res.close()

    def _build_old_user_query(self, uid_range, params, **kwargs):
        if uid_range:
            # construct the range from the passed arguments
            rstr = []
            try:
                if uid_range[0]:
                    rstr.append("uid > :start")
                    params["start"] = uid_range[0]
                if uid_range[1]:
                    rstr.append("uid < :end")
                    params["end"] = uid_range[1]
            except IndexError:
                pass
            rrep = " and ".join(rstr)
            sql = sqltext(
                _GET_OLD_USER_RECORDS_FOR_SERVICE_RANGE.replace("::RANGE::", rrep)
            )
        else:
            sql = _GET_OLD_USER_RECORDS_FOR_SERVICE
        return sql

    def get_old_user_records(
        self, grace_period=-1, limit=100, offset=0, uid_range=None
    ):
        """Get user records that were replaced outside the grace period."""
        if grace_period < 0:
            grace_period = 60 * 60 * 24 * 7  # one week, in seconds
        grace_period = int(grace_period * 1000)  # convert seconds -> millis
        params = {
            "service": self._get_service_id(SERVICE_NAME),
            "timestamp": get_timestamp() - grace_period,
            "limit": limit,
            "offset": offset,
        }

        sql = self._build_old_user_query(uid_range, params)

        res = self._execute_sql(sql, **params)
        try:
            for row in res:
                yield row
        finally:
            res.close()

    def replace_user_records(self, email, timestamp=None):
        """Mark all existing records for a user as replaced."""
        if timestamp is None:
            timestamp = get_timestamp()
        params = {
            "service": self._get_service_id(SERVICE_NAME),
            "email": email,
            "timestamp": timestamp,
        }
        res = self._execute_sql(_REPLACE_USER_RECORDS, **params)
        res.close()

    def replace_user_record(self, uid, timestamp=None):
        """Mark an existing service record as replaced."""
        if timestamp is None:
            timestamp = get_timestamp()
        params = {
            "service": self._get_service_id(SERVICE_NAME),
            "uid": uid,
            "timestamp": timestamp,
        }
        res = self._execute_sql(_REPLACE_USER_RECORD, **params)
        res.close()

    def delete_user_record(self, uid):
        """Delete the user record with the given uid."""
        params = {"service": self._get_service_id(SERVICE_NAME), "uid": uid}
        if not self.spanner_node_id:
            res = self._execute_sql(_FREE_SLOT_ON_NODE, **params)
            res.close()
        res = self._execute_sql(_DELETE_USER_RECORD, **params)
        res.close()

    #
    # Nodes management
    #

    def _get_service_id(self, service):
        if hasattr(self, "service_id"):
            return self.service_id
        else:
            res = self._execute_sql(_GET_SERVICE_ID, service=service)
            row = res.fetchone()
            res.close()
            if row is None:
                raise Exception("unknown service: " + service)
            self.service_id = row.id

            return row.id

    def add_service(self, service_name, pattern, **kwds):
        """Add definition for a new service."""
        if self.db_mode == "postgresql":
            insert_sql = sqltext("""
            insert into services (service, pattern)
            values (:servicename, :pattern)
            RETURNING id
        """)
        else:
            insert_sql = sqltext("""
          insert into services (service, pattern)
          values (:servicename, :pattern)
        """)
        res = self._execute_sql(
            insert_sql,
            servicename=service_name,
            pattern=pattern,
            **kwds,
        )
        res.close()
        if self.db_mode == "postgresql":
            return res.fetchone()[0]
        else:
            return res.lastrowid

    def add_node(self, node, capacity, **kwds):
        """Add definition for a new node."""
        available = kwds.get("available")
        # We release only a fraction of the node's capacity to start.
        if available is None:
            available = math.ceil(capacity * self.capacity_release_rate)
        cols = [
            "service",
            "node",
            "available",
            "capacity",
            "current_load",
            "downed",
            "backoff",
        ]
        args = [":" + v for v in cols]
        # Handle test cases that require nodeid to be 800
        if "nodeid" in kwds:
            cols.append("id")
            args.append(":nodeid")
        query = """
            insert into nodes ({cols})
            values ({args})
            """.format(cols=", ".join(cols), args=", ".join(args))
        res = self._execute_sql(
            sqltext(query),
            nodeid=kwds.get("nodeid"),
            service=self._get_service_id(SERVICE_NAME),
            node=node,
            capacity=capacity,
            available=available,
            current_load=kwds.get("current_load", 0),
            downed=kwds.get("downed", 0),
            backoff=kwds.get("backoff", 0),
        )
        res.close()

    def update_node(self, node, **kwds):
        """Updates node fields in the db."""
        values = {}
        cols = NODE_FIELDS & kwds.keys()
        for col in NODE_FIELDS:
            try:
                values[col] = kwds.pop(col)
            except KeyError:
                pass

        args = [v + " = :" + v for v in cols]
        query = """
            update nodes
            set """
        query += ", ".join(args)
        query += """
            where service = :service and node = :node
        """
        values["service"] = self._get_service_id(SERVICE_NAME)
        values["node"] = node
        if kwds:
            raise ValueError("unknown fields: " + str(kwds.keys()))
        con = self._execute_sql(sqltext(query), **values)
        con.close()

    def get_node_id(self, node):
        """Get numeric id for a node."""
        res = self._execute_sql(
            sqltext("""
            select id from nodes
            where service=:service and node=:node
            """),
            service=self._get_service_id(SERVICE_NAME),
            node=node,
        )
        row = res.fetchone()
        res.close()
        if row is None:
            raise ValueError("unknown node: " + node)
        return row[0]

    def remove_node(self, node, timestamp=None):
        """Remove definition for a node."""
        nodeid = self.get_node_id(node)
        res = self._execute_sql(
            sqltext(
                """
            delete from nodes where id=:nodeid
            """
            ),
            nodeid=nodeid,
        )
        res.close()
        self.unassign_node(node, timestamp, nodeid=nodeid)

    def unassign_node(self, node, timestamp=None, nodeid=None):
        """Clear any assignments to a node."""
        if timestamp is None:
            timestamp = get_timestamp()
        if nodeid is None:
            nodeid = self.get_node_id(node)
        res = self._execute_sql(
            sqltext("""
            update users
            set replaced_at=:timestamp
            where nodeid=:nodeid
            """),
            nodeid=nodeid,
            timestamp=timestamp,
        )
        res.close()

    def get_best_node(self):
        """Returns the 'least loaded' node currently available, increments the
        active count on that node, and decrements the slots currently available
        """
        # The spanner node is the best node.
        if self.spanner_node:
            return self.spanner_node_id, self.spanner_node
        # if, for whatever reason, we haven't gotten the spanner node yet...
        if self.spanner_node_id:
            self.spanner_node = self.get_spanner_node(self.spanner_node_id)
            return self.spanner_node_id, self.spanner_node
        else:
            # We may have to re-try the query if we need to release more
            # capacity.  This loop allows a maximum of five retries before
            # bailing out.
            for _ in range(5):
                res = self._execute_sql(
                    _GET_BEST_NODE, service=self._get_service_id(SERVICE_NAME)
                )
                row = res.fetchone()
                res.close()
                if row is None:
                    # Try to release additional capacity from any nodes
                    # that are not fully occupied.
                    res = self._execute_sql(
                        _RELEASE_NODE_CAPACITY,
                        capacity_release_rate=self.capacity_release_rate,
                        service=self._get_service_id(SERVICE_NAME),
                    )
                    res.close()
                    if res.rowcount == 0:
                        break
                else:
                    break

        # Did we succeed in finding a node?
        if row is None:
            raise Exception("unable to get a node")

        nodeid = row.id
        node = str(row.node)

        # Update the node to reflect the new assignment.
        # This is a little racy with concurrent assignments, but no big
        # deal.
        con = self._execute_sql(
            _ADD_USER_TO_NODE, service=self._get_service_id(SERVICE_NAME), node=node
        )
        con.close()

        return nodeid, node

    def get_node(self, node):
        if node is None:
            raise Exception("NONE node")
        res = self._execute_sql(
            _GET_NODE, service=self._get_service_id(SERVICE_NAME), node=node
        )
        row = res.fetchone()
        res.close()
        if row is None:
            raise Exception("unknown node: " + node)
        return row

    # somewhat simplified version that just gets the one Spanner node.
    def get_spanner_node(self, node):
        res = self._execute_sql(_GET_SPANNER_NODE, id=node)
        row = res.fetchone()
        res.close()
        if row is None:
            raise Exception(f"unknown node: {node}")
        return str(row.node)


def max_keys_changed_at(user, keys_changed_at):
    """Return the largest `keys_changed_at` between the user record and the
    specified value.

    May return `None` as the column is nullable.

    """
    it = (x for x in (keys_changed_at, user["keys_changed_at"]) if x is not None)
    return max(it, default=None)
