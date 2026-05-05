# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Functional tests for the SyncStorage server protocol.

This file runs tests to ensure the correct operation of the server
as specified in:

    http://docs.services.mozilla.com/storage/apis-1.5.html

If there's an aspect of that spec that's not covered by a test in this file,
consider it a bug.
"""

import pytest

import re
import json
import time
import random
import string
import urllib

import simplejson  # type: ignore[import-untyped]

from pyramid.interfaces import IAuthenticationPolicy
from webtest.app import AppError

import tokenlib

from tools.integration_tests.conftest import (
    switch_user,
    retry_post_json,
    retry_put_json,
    retry_delete,
)


WEAVE_INVALID_WBO = 8  # Invalid Weave Basic Object
WEAVE_SIZE_LIMIT_EXCEEDED = 17  # Size limit exceeded

BATCH_MAX_IDS = 100


def get_limit_config(request, limit):
    """Get the configured value for the named size limit."""
    return request.registry.settings["storage." + limit]


def json_dumps(value):
    """Decimal-aware version of json.dumps()."""
    return simplejson.dumps(value, use_decimal=True)


def json_loads(value):
    """Decimal-aware version of json.loads()."""
    return simplejson.loads(value, use_decimal=True)


_PLD = "*" * 500
_ASCII = string.ascii_letters + string.digits


def randtext(size=10):
    """Return a random ASCII string of the given size."""
    return "".join([random.choice(_ASCII) for i in range(size)])


def test_get_info_collections(st_ctx):
    """Test get info collections."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # xxx_col1 gets 3 items, xxx_col2 gets 5 items.
    bsos = [{"id": str(i).zfill(2), "payload": "xxx"} for i in range(3)]
    resp = retry_post_json(app, root + "/storage/xxx_col1", bsos)
    ts1 = resp.json["modified"]
    bsos = [{"id": str(i).zfill(2), "payload": "xxx"} for i in range(5)]
    resp = retry_post_json(app, root + "/storage/xxx_col2", bsos)
    ts2 = resp.json["modified"]
    # only those collections should appear in the query.
    resp = app.get(root + "/info/collections")
    res = resp.json
    keys = sorted(list(res.keys()))
    assert keys == ["xxx_col1", "xxx_col2"]
    assert res["xxx_col1"] == ts1
    assert res["xxx_col2"] == ts2
    # Updating items in xxx_col2, check timestamps.
    bsos = [{"id": str(i).zfill(2), "payload": "yyy"} for i in range(2)]
    resp = retry_post_json(app, root + "/storage/xxx_col2", bsos)
    assert ts2 < resp.json["modified"]
    ts2 = resp.json["modified"]
    resp = app.get(root + "/info/collections")
    res = resp.json
    keys = sorted(list(res.keys()))
    assert keys == ["xxx_col1", "xxx_col2"]
    assert res["xxx_col1"] == ts1
    assert res["xxx_col2"] == ts2


def test_get_collection_count(st_ctx):
    """Test get collection count."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # xxx_col1 gets 3 items, xxx_col2 gets 5 items.
    bsos = [{"id": str(i).zfill(2), "payload": "xxx"} for i in range(3)]
    retry_post_json(app, root + "/storage/xxx_col1", bsos)
    bsos = [{"id": str(i).zfill(2), "payload": "xxx"} for i in range(5)]
    retry_post_json(app, root + "/storage/xxx_col2", bsos)
    # those counts should be reflected back in query.
    resp = app.get(root + "/info/collection_counts")
    res = resp.json
    assert len(res) == 2
    assert res["xxx_col1"] == 3
    assert res["xxx_col2"] == 5


def test_bad_cache(st_ctx):
    """Test bad cache."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # fixes #637332
    # the collection name <-> id mapper is temporarely cached to
    # save a few requests.
    # but should get purged when new collections are added

    # 1. get collection info
    resp = app.get(root + "/info/collections")
    numcols = len(resp.json)

    # 2. add a new collection + stuff
    bso = {"id": "125", "payload": _PLD}
    retry_put_json(app, root + "/storage/xxxx/125", bso)

    # 3. get collection info again, should find the new ones
    resp = app.get(root + "/info/collections")
    assert len(resp.json) == numcols + 1


def test_get_collection_only(st_ctx):
    """Test get collection only."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    bsos = [{"id": str(i).zfill(2), "payload": "xxx"} for i in range(5)]
    retry_post_json(app, root + "/storage/xxx_col2", bsos)

    # non-existent collections appear as empty
    resp = app.get(root + "/storage/nonexistent")
    res = resp.json
    assert res == []

    # try just getting all items at once.
    resp = app.get(root + "/storage/xxx_col2")
    res = resp.json
    res.sort()
    assert res == ["00", "01", "02", "03", "04"]
    assert int(resp.headers["X-Weave-Records"]) == 5

    # trying various filters

    # "ids"
    # Returns the ids for objects in the collection that are in the
    # provided comma-separated list.
    res = app.get(root + "/storage/xxx_col2?ids=01,03,17")
    res = res.json
    res.sort()
    assert res == ["01", "03"]

    # "newer"
    # Returns only ids for objects in the collection that have been last
    # modified after the timestamp given.

    retry_delete(app, root + "/storage/xxx_col2")

    bso = {"id": "128", "payload": "x"}
    res = retry_put_json(app, root + "/storage/xxx_col2/128", bso)
    ts1 = float(res.headers["X-Last-Modified"])

    bso = {"id": "129", "payload": "x"}
    res = retry_put_json(app, root + "/storage/xxx_col2/129", bso)
    ts2 = float(res.headers["X-Last-Modified"])

    assert ts1 < ts2

    res = app.get(root + "/storage/xxx_col2?newer=%s" % ts1)
    assert res.json == ["129"]

    res = app.get(root + "/storage/xxx_col2?newer=%s" % ts2)
    assert res.json == []

    res = app.get(root + "/storage/xxx_col2?newer=%s" % (ts1 - 1))
    assert sorted(res.json) == ["128", "129"]

    # "older"
    # Returns only ids for objects in the collection that have been last
    # modified before the timestamp given.

    retry_delete(app, root + "/storage/xxx_col2")

    bso = {"id": "128", "payload": "x"}
    res = retry_put_json(app, root + "/storage/xxx_col2/128", bso)
    ts1 = float(res.headers["X-Last-Modified"])

    bso = {"id": "129", "payload": "x"}
    res = retry_put_json(app, root + "/storage/xxx_col2/129", bso)
    ts2 = float(res.headers["X-Last-Modified"])

    assert ts1 < ts2

    res = app.get(root + "/storage/xxx_col2?older=%s" % ts1)
    assert res.json == []

    res = app.get(root + "/storage/xxx_col2?older=%s" % ts2)
    assert res.json == ["128"]

    res = app.get(root + "/storage/xxx_col2?older=%s" % (ts2 + 1))
    assert sorted(res.json) == ["128", "129"]

    qs = "?older=%s&newer=%s" % (ts2 + 1, ts1)
    res = app.get(root + "/storage/xxx_col2" + qs)
    assert sorted(res.json) == ["129"]

    # "full"
    # If defined, returns the full BSO, rather than just the id.
    res = app.get(root + "/storage/xxx_col2?full=1")
    keys = list(res.json[0].keys())
    keys.sort()
    wanted = ["id", "modified", "payload"]
    assert keys == wanted

    res = app.get(root + "/storage/xxx_col2")
    assert isinstance(res.json, list)

    # "limit"
    # Sets the maximum number of ids that will be returned
    retry_delete(app, root + "/storage/xxx_col2")

    bsos = []
    for i in range(10):
        bso = {"id": str(i).zfill(2), "payload": "x", "sortindex": i}
        bsos.append(bso)
    retry_post_json(app, root + "/storage/xxx_col2", bsos)

    query_url = root + "/storage/xxx_col2?sort=index"
    res = app.get(query_url)
    all_items = res.json
    assert len(all_items) == 10

    res = app.get(query_url + "&limit=2")
    assert res.json == all_items[:2]

    # "offset"
    # Skips over items that have already been returned.
    next_offset = res.headers["X-Weave-Next-Offset"]
    res = app.get(query_url + "&limit=3&offset=" + next_offset)
    assert res.json == all_items[2:5]

    next_offset = res.headers["X-Weave-Next-Offset"]
    res = app.get(query_url + "&offset=" + next_offset)
    assert res.json == all_items[5:]
    assert "X-Weave-Next-Offset" not in res.headers

    res = app.get(query_url + "&limit=10000&offset=" + next_offset)
    assert res.json == all_items[5:]
    assert "X-Weave-Next-Offset" not in res.headers

    # "offset" again, this time ordering by descending timestamp.
    query_url = root + "/storage/xxx_col2?sort=newest"
    res = app.get(query_url)
    all_items = res.json
    assert len(all_items) == 10

    res = app.get(query_url + "&limit=2")
    assert res.json == all_items[:2]

    next_offset = res.headers["X-Weave-Next-Offset"]
    res = app.get(query_url + "&limit=3&offset=" + next_offset)
    assert res.json == all_items[2:5]

    next_offset = res.headers["X-Weave-Next-Offset"]
    res = app.get(query_url + "&offset=" + next_offset)
    assert res.json == all_items[5:]
    assert "X-Weave-Next-Offset" not in res.headers

    res = app.get(query_url + "&limit=10000&offset=" + next_offset)
    assert res.json == all_items[5:]

    # "offset" again, this time ordering by ascending timestamp.
    query_url = root + "/storage/xxx_col2?sort=oldest"
    res = app.get(query_url)
    all_items = res.json
    assert len(all_items) == 10

    res = app.get(query_url + "&limit=2")
    assert res.json == all_items[:2]

    next_offset = res.headers["X-Weave-Next-Offset"]
    res = app.get(query_url + "&limit=3&offset=" + next_offset)
    assert res.json == all_items[2:5]

    next_offset = res.headers["X-Weave-Next-Offset"]
    res = app.get(query_url + "&offset=" + next_offset)
    assert res.json == all_items[5:]
    assert "X-Weave-Next-Offset" not in res.headers

    res = app.get(query_url + "&limit=10000&offset=" + next_offset)
    assert res.json == all_items[5:]

    # "offset" once more, this time with no explicit ordering
    query_url = root + "/storage/xxx_col2?"
    res = app.get(query_url)
    all_items = res.json
    assert len(all_items) == 10

    res = app.get(query_url + "&limit=2")
    assert res.json == all_items[:2]

    next_offset = res.headers["X-Weave-Next-Offset"]
    res = app.get(query_url + "&limit=3&offset=" + next_offset)
    assert res.json == all_items[2:5]

    next_offset = res.headers["X-Weave-Next-Offset"]
    res = app.get(query_url + "&offset=" + next_offset)
    assert res.json == all_items[5:]
    assert "X-Weave-Next-Offset" not in res.headers

    res = app.get(query_url + "&limit=10000&offset=" + next_offset)

    # "sort"
    #   'newest': Orders by timestamp number (newest first)
    #   'oldest': Orders by timestamp number (oldest first)
    #   'index':  Orders by the sortindex descending (highest weight first)
    retry_delete(app, root + "/storage/xxx_col2")

    for index, sortindex in (("00", -1), ("01", 34), ("02", 12)):
        bso = {"id": index, "payload": "x", "sortindex": sortindex}
        retry_post_json(app, root + "/storage/xxx_col2", [bso])

    res = app.get(root + "/storage/xxx_col2?sort=newest")
    res = res.json
    assert res == ["02", "01", "00"]

    res = app.get(root + "/storage/xxx_col2?sort=oldest")
    res = res.json
    assert res == ["00", "01", "02"]

    res = app.get(root + "/storage/xxx_col2?sort=index")
    res = res.json
    assert res == ["01", "02", "00"]


def test_alternative_formats(st_ctx):
    """Test alternative formats."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    bsos = [{"id": str(i).zfill(2), "payload": "xxx"} for i in range(5)]
    retry_post_json(app, root + "/storage/xxx_col2", bsos)

    # application/json
    res = app.get(
        root + "/storage/xxx_col2",
        headers=[("Accept", "application/json")],
    )
    assert res.content_type.split(";")[0] == "application/json"

    res = res.json
    res.sort()
    assert res == ["00", "01", "02", "03", "04"]

    # application/newlines
    res = app.get(
        root + "/storage/xxx_col2",
        headers=[("Accept", "application/newlines")],
    )
    assert res.content_type == "application/newlines"

    assert res.body.endswith(b"\n")
    res = [json_loads(line) for line in res.body.decode("utf-8").strip().split("\n")]
    res.sort()
    assert res == ["00", "01", "02", "03", "04"]

    # unspecified format defaults to json
    res = app.get(root + "/storage/xxx_col2")
    assert res.content_type.split(";")[0] == "application/json"

    # unkown format gets a 406
    app.get(
        root + "/storage/xxx_col2",
        headers=[("Accept", "x/yy")],
        status=406,
    )


def test_set_collection_with_if_modified_since(st_ctx):
    """Test set collection with if modified since."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # Create five items with different timestamps.
    for i in range(5):
        bsos = [{"id": str(i).zfill(2), "payload": "xxx"}]
        retry_post_json(app, root + "/storage/xxx_col2", bsos)
    # Get them all, along with their timestamps.
    res = app.get(root + "/storage/xxx_col2?full=true").json
    assert len(res) == 5
    timestamps = sorted([r["modified"] for r in res])
    # The timestamp of the collection should be the max of all those.
    app.get(
        root + "/storage/xxx_col2",
        headers={"X-If-Modified-Since": str(timestamps[0])},
        status=200,
    )
    res = app.get(
        root + "/storage/xxx_col2",
        headers={"X-If-Modified-Since": str(timestamps[-1])},
        status=304,
    )
    assert "X-Last-Modified" in res.headers


def test_get_item(st_ctx):
    """Test get item."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    bsos = [{"id": str(i).zfill(2), "payload": "xxx"} for i in range(5)]
    retry_post_json(app, root + "/storage/xxx_col2", bsos)
    # grabbing object 1 from xxx_col2
    res = app.get(root + "/storage/xxx_col2/01")
    res = res.json
    keys = list(res.keys())
    keys.sort()
    assert keys == ["id", "modified", "payload"]
    assert res["id"] == "01"

    # unexisting object
    app.get(root + "/storage/xxx_col2/99", status=404)

    # using x-if-modified-since header.
    app.get(
        root + "/storage/xxx_col2/01",
        headers={"X-If-Modified-Since": str(res["modified"])},
        status=304,
    )
    app.get(
        root + "/storage/xxx_col2/01",
        headers={"X-If-Modified-Since": str(res["modified"] + 1)},
        status=304,
    )
    res = app.get(
        root + "/storage/xxx_col2/01",
        headers={"X-If-Modified-Since": str(res["modified"] - 1)},
    )
    assert res.json["id"] == "01"


def test_set_item(st_ctx):
    """Test set item."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # let's create an object
    bso = {"payload": _PLD}
    retry_put_json(app, root + "/storage/xxx_col2/12345", bso)
    res = app.get(root + "/storage/xxx_col2/12345")
    res = res.json
    assert res["payload"] == _PLD

    # now let's update it
    bso = {"payload": "YYY"}
    retry_put_json(app, root + "/storage/xxx_col2/12345", bso)
    res = app.get(root + "/storage/xxx_col2/12345")
    res = res.json
    assert res["payload"] == "YYY"


def test_set_collection(st_ctx):
    """Test set collection."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # sending two bsos
    bso1 = {"id": "12", "payload": _PLD}
    bso2 = {"id": "13", "payload": _PLD}
    bsos = [bso1, bso2]
    retry_post_json(app, root + "/storage/xxx_col2", bsos)

    # checking what we did
    res = app.get(root + "/storage/xxx_col2/12")
    res = res.json
    assert res["payload"] == _PLD
    res = app.get(root + "/storage/xxx_col2/13")
    res = res.json
    assert res["payload"] == _PLD

    # one more time, with changes
    bso1 = {"id": "13", "payload": "XyX"}
    bso2 = {"id": "14", "payload": _PLD}
    bsos = [bso1, bso2]
    retry_post_json(app, root + "/storage/xxx_col2", bsos)

    # checking what we did
    res = app.get(root + "/storage/xxx_col2/14")
    res = res.json
    assert res["payload"] == _PLD
    res = app.get(root + "/storage/xxx_col2/13")
    res = res.json
    assert res["payload"] == "XyX"

    # sending two bsos with one bad sortindex
    bso1 = {"id": "one", "payload": _PLD}
    bso2 = {"id": "two", "payload": _PLD, "sortindex": "FAIL"}
    bsos = [bso1, bso2]
    retry_post_json(app, root + "/storage/xxx_col2", bsos)
    app.get(root + "/storage/xxx_col2/two", status=404)


def test_set_collection_input_formats(st_ctx):
    """Test set collection input formats."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # If we send with application/newlines it should work.
    bso1 = {"id": "12", "payload": _PLD}
    bso2 = {"id": "13", "payload": _PLD}
    bsos = [bso1, bso2]
    body = "\n".join(json_dumps(bso) for bso in bsos)
    app.post(
        root + "/storage/xxx_col2",
        body,
        headers={"Content-Type": "application/newlines"},
    )
    items = app.get(root + "/storage/xxx_col2").json
    assert len(items) == 2
    # If we send an unknown content type, we get an error.
    retry_delete(app, root + "/storage/xxx_col2")
    body = json_dumps(bsos)
    app.post(
        root + "/storage/xxx_col2",
        body,
        headers={"Content-Type": "application/octet-stream"},
        status=415,
    )
    items = app.get(root + "/storage/xxx_col2").json
    assert len(items) == 0


def test_set_item_input_formats(st_ctx):
    """Test set item input formats."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # If we send with application/json it should work.
    body = json_dumps({"payload": _PLD})
    app.put(
        root + "/storage/xxx_col2/TEST",
        body,
        headers={"Content-Type": "application/json"},
    )
    item = app.get(root + "/storage/xxx_col2/TEST").json
    assert item["payload"] == _PLD
    # If we send json with some other content type, it should fail
    retry_delete(app, root + "/storage/xxx_col2")
    app.put(
        root + "/storage/xxx_col2/TEST",
        body,
        headers={"Content-Type": "application/octet-stream"},
        status=415,
    )
    app.get(root + "/storage/xxx_col2/TEST", status=404)
    # Unless we use text/plain, which is a special bw-compat case.
    app.put(
        root + "/storage/xxx_col2/TEST",
        body,
        headers={"Content-Type": "text/plain"},
    )
    item = app.get(root + "/storage/xxx_col2/TEST").json
    assert item["payload"] == _PLD


def test_app_newlines_when_payloads_contain_newlines(st_ctx):
    """Test app newlines when payloads contain newlines."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # Send some application/newlines with embedded newline chars.
    bsos = [
        {"id": "01", "payload": "hello\nworld"},
        {"id": "02", "payload": "\nmarco\npolo\n"},
    ]
    body = "\n".join(json_dumps(bso) for bso in bsos)
    assert len(body.split("\n")) == 2
    app.post(
        root + "/storage/xxx_col2",
        body,
        headers={"Content-Type": "application/newlines"},
    )
    # Read them back as JSON list, check payloads.
    items = app.get(root + "/storage/xxx_col2?full=1").json
    assert len(items) == 2
    items.sort(key=lambda bso: bso["id"])
    assert items[0]["payload"] == bsos[0]["payload"]
    assert items[1]["payload"] == bsos[1]["payload"]
    # Read them back as application/newlines, check payloads.
    res = app.get(
        root + "/storage/xxx_col2?full=1",
        headers={
            "Accept": "application/newlines",
        },
    )
    items = [json_loads(line) for line in res.body.decode("utf-8").strip().split("\n")]
    assert len(items) == 2
    items.sort(key=lambda bso: bso["id"])
    assert items[0]["payload"] == bsos[0]["payload"]
    assert items[1]["payload"] == bsos[1]["payload"]


def test_collection_usage(st_ctx):
    """Test collection usage."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    retry_delete(app, root + "/storage")

    bso1 = {"id": "13", "payload": "XyX"}
    bso2 = {"id": "14", "payload": _PLD}
    bsos = [bso1, bso2]
    retry_post_json(app, root + "/storage/xxx_col2", bsos)

    res = app.get(root + "/info/collection_usage")
    usage = res.json
    xxx_col2_size = usage["xxx_col2"]
    wanted = (len(bso1["payload"]) + len(bso2["payload"])) / 1024.0
    assert round(xxx_col2_size, 2) == round(wanted, 2)


def test_delete_collection_items(st_ctx):
    """Test delete collection items."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # creating a collection of three
    bso1 = {"id": "12", "payload": _PLD}
    bso2 = {"id": "13", "payload": _PLD}
    bso3 = {"id": "14", "payload": _PLD}
    bsos = [bso1, bso2, bso3]
    retry_post_json(app, root + "/storage/xxx_col2", bsos)
    res = app.get(root + "/storage/xxx_col2")
    assert len(res.json) == 3

    # deleting all items
    retry_delete(app, root + "/storage/xxx_col2")
    items = app.get(root + "/storage/xxx_col2").json
    assert len(items) == 0

    # Deletes the ids for objects in the collection that are in the
    # provided comma-separated list.
    retry_post_json(app, root + "/storage/xxx_col2", bsos)
    res = app.get(root + "/storage/xxx_col2")
    assert len(res.json) == 3
    retry_delete(app, root + "/storage/xxx_col2?ids=12,14")
    res = app.get(root + "/storage/xxx_col2")
    assert len(res.json) == 1
    retry_delete(app, root + "/storage/xxx_col2?ids=13")
    res = app.get(root + "/storage/xxx_col2")
    assert len(res.json) == 0


def test_delete_item(st_ctx):
    """Test delete item."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # creating a collection of three
    bso1 = {"id": "12", "payload": _PLD}
    bso2 = {"id": "13", "payload": _PLD}
    bso3 = {"id": "14", "payload": _PLD}
    bsos = [bso1, bso2, bso3]
    retry_post_json(app, root + "/storage/xxx_col2", bsos)
    res = app.get(root + "/storage/xxx_col2")
    assert len(res.json) == 3
    ts = float(res.headers["X-Last-Modified"])

    # deleting item 13
    retry_delete(app, root + "/storage/xxx_col2/13")
    res = app.get(root + "/storage/xxx_col2")
    assert len(res.json) == 2

    # unexisting item should return a 404
    retry_delete(app, root + "/storage/xxx_col2/12982", status=404)

    # The collection should get an updated timestsamp.
    res = app.get(root + "/info/collections")
    assert ts < float(res.headers["X-Last-Modified"])


def test_delete_storage(st_ctx):
    """Test delete storage."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # creating a collection of three
    bso1 = {"id": "12", "payload": _PLD}
    bso2 = {"id": "13", "payload": _PLD}
    bso3 = {"id": "14", "payload": _PLD}
    bsos = [bso1, bso2, bso3]
    retry_post_json(app, root + "/storage/xxx_col2", bsos)
    res = app.get(root + "/storage/xxx_col2")
    assert len(res.json) == 3

    # deleting all
    retry_delete(app, root + "/storage")
    items = app.get(root + "/storage/xxx_col2").json
    assert len(items) == 0
    retry_delete(app, root + "/storage/xxx_col2", status=200)
    assert len(items) == 0


def test_x_timestamp_header(st_ctx):
    """Test x timestamp header."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    bsos = [{"id": str(i).zfill(2), "payload": "xxx"} for i in range(5)]
    retry_post_json(app, root + "/storage/xxx_col2", bsos)

    now = round(time.time(), 2)
    time.sleep(0.01)
    res = app.get(root + "/storage/xxx_col2")
    assert now <= float(res.headers["X-Weave-Timestamp"])

    # getting the timestamp with a PUT
    now = round(time.time(), 2)
    time.sleep(0.01)
    bso = {"payload": _PLD}
    res = retry_put_json(app, root + "/storage/xxx_col2/12345", bso)
    assert now <= float(res.headers["X-Weave-Timestamp"])
    assert abs(now - float(res.headers["X-Weave-Timestamp"])) <= 200

    # getting the timestamp with a POST
    now = round(time.time(), 2)
    time.sleep(0.01)
    bso1 = {"id": "12", "payload": _PLD}
    bso2 = {"id": "13", "payload": _PLD}
    bsos = [bso1, bso2]
    res = retry_post_json(app, root + "/storage/xxx_col2", bsos)
    assert now <= float(res.headers["X-Weave-Timestamp"])


def test_ifunmodifiedsince(st_ctx):
    """Test ifunmodifiedsince."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    bso = {"id": "12345", "payload": _PLD}
    res = retry_put_json(app, root + "/storage/xxx_col2/12345", bso)
    # Using an X-If-Unmodified-Since in the past should cause 412s.
    ts = str(float(res.headers["X-Last-Modified"]) - 1)
    bso = {"id": "12345", "payload": _PLD + "XXX"}
    res = retry_put_json(
        app,
        root + "/storage/xxx_col2/12345",
        bso,
        headers=[("X-If-Unmodified-Since", ts)],
        status=412,
    )
    assert "X-Last-Modified" in res.headers
    res = retry_delete(
        app,
        root + "/storage/xxx_col2/12345",
        headers=[("X-If-Unmodified-Since", ts)],
        status=412,
    )
    assert "X-Last-Modified" in res.headers
    retry_post_json(
        app,
        root + "/storage/xxx_col2",
        [bso],
        headers=[("X-If-Unmodified-Since", ts)],
        status=412,
    )
    retry_delete(
        app,
        root + "/storage/xxx_col2?ids=12345",
        headers=[("X-If-Unmodified-Since", ts)],
        status=412,
    )
    app.get(
        root + "/storage/xxx_col2/12345",
        headers=[("X-If-Unmodified-Since", ts)],
        status=412,
    )
    app.get(
        root + "/storage/xxx_col2",
        headers=[("X-If-Unmodified-Since", ts)],
        status=412,
    )
    # Deleting items from a collection should give 412 even if some
    # other, unrelated item in the collection has been modified.
    ts = res.headers["X-Last-Modified"]
    res2 = retry_put_json(
        app,
        root + "/storage/xxx_col2/54321",
        {
            "payload": _PLD,
        },
    )
    retry_delete(
        app,
        root + "/storage/xxx_col2?ids=12345",
        headers=[("X-If-Unmodified-Since", ts)],
        status=412,
    )
    ts = res2.headers["X-Last-Modified"]
    # All of those should have left the BSO unchanged
    res2 = app.get(root + "/storage/xxx_col2/12345")
    assert res2.json["payload"] == _PLD
    assert res2.headers["X-Last-Modified"] == res.headers["X-Last-Modified"]
    # Using an X-If-Unmodified-Since equal to
    # X-Last-Modified should allow the request to succeed.
    res = retry_post_json(
        app,
        root + "/storage/xxx_col2",
        [bso],
        headers=[("X-If-Unmodified-Since", ts)],
        status=200,
    )
    ts = res.headers["X-Last-Modified"]
    app.get(
        root + "/storage/xxx_col2/12345",
        headers=[("X-If-Unmodified-Since", ts)],
        status=200,
    )
    retry_delete(
        app,
        root + "/storage/xxx_col2/12345",
        headers=[("X-If-Unmodified-Since", ts)],
        status=200,
    )
    res = retry_put_json(
        app,
        root + "/storage/xxx_col2/12345",
        bso,
        headers=[("X-If-Unmodified-Since", "0")],
        status=200,
    )
    ts = res.headers["X-Last-Modified"]
    app.get(
        root + "/storage/xxx_col2",
        headers=[("X-If-Unmodified-Since", ts)],
        status=200,
    )
    retry_delete(
        app,
        root + "/storage/xxx_col2?ids=12345",
        headers=[("X-If-Unmodified-Since", ts)],
        status=200,
    )


def test_quota(st_ctx):
    """Test quota."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    res = app.get(root + "/info/quota")
    old_used = res.json[0]
    bso = {"payload": _PLD}
    retry_put_json(app, root + "/storage/xxx_col2/12345", bso)
    res = app.get(root + "/info/quota")
    used = res.json[0]
    assert used - old_used == len(_PLD) / 1024.0


def test_get_collection_ttl(st_ctx):
    """Test get collection ttl."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    bso = {"payload": _PLD, "ttl": 0}
    retry_put_json(app, root + "/storage/xxx_col2/12345", bso)
    time.sleep(1.1)
    res = app.get(root + "/storage/xxx_col2")
    assert res.json == []

    bso = {"payload": _PLD, "ttl": 2}
    retry_put_json(app, root + "/storage/xxx_col2/123456", bso)

    # it should exists now
    res = app.get(root + "/storage/xxx_col2")
    assert len(res.json) == 1

    # trying a second put again
    retry_put_json(app, root + "/storage/xxx_col2/123456", bso)

    res = app.get(root + "/storage/xxx_col2")
    assert len(res.json) == 1
    time.sleep(2.1)
    res = app.get(root + "/storage/xxx_col2")
    assert len(res.json) == 0


def test_multi_item_post_limits(st_ctx):
    """Test multi item post limits."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    res = app.get(root + "/info/configuration")
    try:
        max_bytes = res.json["max_post_bytes"]
        max_count = res.json["max_post_records"]
        max_req_bytes = res.json["max_request_bytes"]
    except KeyError:
        max_bytes = get_limit_config(st_ctx["config"], "max_post_bytes")
        max_count = get_limit_config(st_ctx["config"], "max_post_records")
        max_req_bytes = get_limit_config(st_ctx["config"], "max_request_bytes")

    # Uploading max_count-5 small objects should succeed.
    bsos = [{"id": str(i).zfill(2), "payload": "X"} for i in range(max_count - 5)]
    res = retry_post_json(app, root + "/storage/xxx_col2", bsos)
    res = res.json
    assert len(res["success"]) == max_count - 5
    assert len(res["failed"]) == 0

    # Uploading max_count+5 items should produce five failures.
    bsos = [{"id": str(i).zfill(2), "payload": "X"} for i in range(max_count + 5)]
    res = retry_post_json(app, root + "/storage/xxx_col2", bsos)
    res = res.json
    assert len(res["success"]) == max_count
    assert len(res["failed"]) == 5

    # Uploading items such that the last item puts us over the
    # cumulative limit on payload size, should produce 1 failure.
    # The item_size here is arbitrary, so I made it a prime in kB.
    item_size = 227 * 1024
    max_items, leftover = divmod(max_bytes, item_size)
    bsos = [
        {"id": str(i).zfill(2), "payload": "X" * item_size} for i in range(max_items)
    ]
    bsos.append({"id": str(max_items), "payload": "X" * (leftover + 1)})

    # Check that we don't go over the limit on raw request bytes,
    # which would get us rejected in production with a 413.
    assert len(json.dumps(bsos)) < max_req_bytes

    res = retry_post_json(app, root + "/storage/xxx_col2", bsos)
    res = res.json
    assert len(res["success"]) == max_items
    assert len(res["failed"]) == 1


def test_weird_args(st_ctx):
    """Test weird args."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # pushing some data in xxx_col2
    bsos = [{"id": str(i).zfill(2), "payload": _PLD} for i in range(10)]
    res = retry_post_json(app, root + "/storage/xxx_col2", bsos)
    res = res.json

    # trying weird args and make sure the server returns 400s
    # Note: "Offset" is a string since the bsoid could be anything.
    # skipping that for now.
    args = ("newer", "older", "limit")
    for arg in args:
        value = randtext()
        app.get(
            root + "/storage/xxx_col2?%s=%s" % (arg, value),
            status=400,
        )

    # what about a crazy ids= string ?
    ids = ",".join([randtext(10) for i in range(100)])
    res = app.get(root + "/storage/xxx_col2?ids=%s" % ids)
    assert res.json == []

    # trying unexpected args - they should not break
    app.get(root + "/storage/xxx_col2?blabla=1", status=200)


def test_guid_deletion(st_ctx):
    """Test guid deletion."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # pushing some data in xxx_col2
    bsos = [
        {
            "id": "6820f3ca-6e8a-4ff4-8af7-8b3625d7d65%d" % i,
            "payload": _PLD,
        }
        for i in range(5)
    ]
    res = retry_post_json(app, root + "/storage/passwords", bsos)
    res = res.json
    assert len(res["success"]) == 5

    # now deleting some of them
    ids = ",".join(["6820f3ca-6e8a-4ff4-8af7-8b3625d7d65%d" % i for i in range(2)])

    retry_delete(app, root + "/storage/passwords?ids=%s" % ids)

    res = app.get(root + "/storage/passwords?ids=%s" % ids)
    assert len(res.json) == 0
    res = app.get(root + "/storage/passwords")
    assert len(res.json) == 3


def test_specifying_ids_with_percent_encoded_query_string(st_ctx):
    """Test specifying ids with percent encoded query string."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # create some items
    bsos = [{"id": "test-%d" % i, "payload": _PLD} for i in range(5)]
    res = retry_post_json(app, root + "/storage/xxx_col2", bsos)
    res = res.json
    assert len(res["success"]) == 5
    # now delete some of them
    ids = ",".join(["test-%d" % i for i in range(2)])
    ids = urllib.request.quote(ids)
    retry_delete(app, root + "/storage/xxx_col2?ids=%s" % ids)
    # check that the correct items were deleted
    res = app.get(root + "/storage/xxx_col2?ids=%s" % ids)
    assert len(res.json) == 0
    res = app.get(root + "/storage/xxx_col2")
    assert len(res.json) == 3


def test_timestamp_numbers_are_decimals(st_ctx):
    """Test timestamp numbers are decimals."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # Create five items with different timestamps.
    for i in range(5):
        bsos = [{"id": str(i).zfill(2), "payload": "xxx"}]
        retry_post_json(app, root + "/storage/xxx_col2", bsos)

    # make sure the server returns only proper precision timestamps.
    resp = app.get(root + "/storage/xxx_col2?full=1")
    bsos = json_loads(resp.body)
    timestamps = []
    for bso in bsos:
        ts = bso["modified"]
        # timestamps could be on the hundred seconds (.10) or on the
        # second (.0) and the zero could be dropped. We just don't want
        # anything beyond milisecond.
        assert len(str(ts).split(".")[-1]) <= 2
        timestamps.append(ts)

    timestamps.sort()

    # try a newer filter now, to get the last two objects
    ts = float(timestamps[-3])

    # Returns only ids for objects in the collection that have been
    # last modified since the timestamp given.
    res = app.get(root + "/storage/xxx_col2?newer=%s" % ts)
    res = res.json
    try:
        assert sorted(res) == ["03", "04"]
    except AssertionError:
        # need to display the whole collection to understand the issue
        msg = "Timestamp used: %s" % ts
        msg += " " + app.get(root + "/storage/xxx_col2?full=1").body
        msg += " Timestamps received: %s" % str(timestamps)
        msg += " Result of newer query: %s" % res
        raise AssertionError(msg)


def test_strict_newer(st_ctx):
    """Test strict newer."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # send two bsos in the 'meh' collection
    bso1 = {"id": "01", "payload": _PLD}
    bso2 = {"id": "02", "payload": _PLD}
    bsos = [bso1, bso2]
    res = retry_post_json(app, root + "/storage/xxx_meh", bsos)
    ts = float(res.headers["X-Last-Modified"])

    # send two more bsos
    bso3 = {"id": "03", "payload": _PLD}
    bso4 = {"id": "04", "payload": _PLD}
    bsos = [bso3, bso4]
    retry_post_json(app, root + "/storage/xxx_meh", bsos)

    # asking for bsos using newer=ts where newer is the timestamp
    # of bso 1 and 2, should not return them
    res = app.get(root + "/storage/xxx_meh?newer=%s" % ts)
    res = res.json
    assert sorted(res) == ["03", "04"]


def test_strict_older(st_ctx):
    """Test strict older."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # send two bsos in the 'xxx_meh' collection
    bso1 = {"id": "01", "payload": _PLD}
    bso2 = {"id": "02", "payload": _PLD}
    bsos = [bso1, bso2]
    retry_post_json(app, root + "/storage/xxx_meh", bsos)

    # send two more bsos
    bso3 = {"id": "03", "payload": _PLD}
    bso4 = {"id": "04", "payload": _PLD}
    bsos = [bso3, bso4]
    res = retry_post_json(app, root + "/storage/xxx_meh", bsos)
    ts = float(res.headers["X-Last-Modified"])

    # asking for bsos using older=ts where older is the timestamp
    # of bso 3 and 4, should not return them
    res = app.get(root + "/storage/xxx_meh?older=%s" % ts)
    res = res.json
    assert sorted(res) == ["01", "02"]


def test_handling_of_invalid_json_in_bso_uploads(st_ctx):
    """Test handling of invalid json in bso uploads."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # Single upload with JSON that's not a BSO.
    bso = "notabso"
    res = retry_put_json(app, root + "/storage/xxx_col2/invalid", bso, status=400)
    assert res.json == WEAVE_INVALID_WBO

    bso = 42
    res = retry_put_json(app, root + "/storage/xxx_col2/invalid", bso, status=400)
    assert res.json == WEAVE_INVALID_WBO

    bso = {"id": ["01", "02"], "payload": {"3": "4"}}
    res = retry_put_json(app, root + "/storage/xxx_col2/invalid", bso, status=400)
    assert res.json == WEAVE_INVALID_WBO

    # Batch upload with JSON that's not a list of BSOs
    bsos = "notalist"
    res = retry_post_json(app, root + "/storage/xxx_col2", bsos, status=400)
    assert res.json == WEAVE_INVALID_WBO

    bsos = 42
    res = retry_post_json(app, root + "/storage/xxx_col2", bsos, status=400)
    assert res.json == WEAVE_INVALID_WBO

    # Batch upload a list with something that's not a valid data dict.
    # It should fail out entirely, as the input is seriously broken.
    bsos = [{"id": "01", "payload": "GOOD"}, "BAD"]
    res = retry_post_json(app, root + "/storage/xxx_col2", bsos, status=400)

    # Batch upload a list with something that's an invalid BSO.
    # It should process the good entry and fail for the bad.
    bsos = [{"id": "01", "payload": "GOOD"}, {"id": "02", "invalid": "ya"}]
    res = retry_post_json(app, root + "/storage/xxx_col2", bsos)
    res = res.json
    assert len(res["success"]) == 1
    assert len(res["failed"]) == 1


def test_handling_of_invalid_bso_fields(st_ctx):
    """Test handling of invalid bso fields."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    coll_url = root + "/storage/xxx_col2"
    # Invalid ID - unacceptable characters.
    # The newline cases are especially nuanced because \n
    # gets special treatment from the regex library.
    bso = {"id": "A\nB", "payload": "testing"}
    res = retry_post_json(app, coll_url, [bso])
    assert res.json["failed"] and not res.json["success"]
    bso = {"id": "A\n", "payload": "testing"}
    res = retry_post_json(app, coll_url, [bso])
    assert res.json["failed"] and not res.json["success"]
    bso = {"id": "\nN", "payload": "testing"}
    res = retry_post_json(app, coll_url, [bso])
    assert res.json["failed"] and not res.json["success"]
    bso = {"id": "A\tB", "payload": "testing"}
    res = retry_post_json(app, coll_url, [bso])
    assert res.json["failed"] and not res.json["success"]
    # Invalid ID - empty string is not acceptable.
    bso = {"id": "", "payload": "testing"}
    res = retry_post_json(app, coll_url, [bso])
    assert res.json["failed"] and not res.json["success"]
    # Invalid ID - too long
    bso = {"id": "X" * 65, "payload": "testing"}
    res = retry_post_json(app, coll_url, [bso])
    assert res.json["failed"] and not res.json["success"]
    # Invalid sortindex - not an integer
    bso = {"id": "TEST", "payload": "testing", "sortindex": "xxx_meh"}
    res = retry_post_json(app, coll_url, [bso])
    assert res.json["failed"] and not res.json["success"]
    res = retry_put_json(app, coll_url + "/" + bso["id"], bso, status=400)
    assert res.json == WEAVE_INVALID_WBO
    # Invalid sortindex - not an integer
    bso = {"id": "TEST", "payload": "testing", "sortindex": "2.6"}
    res = retry_post_json(app, coll_url, [bso])
    assert res.json["failed"] and not res.json["success"]
    res = retry_put_json(app, coll_url + "/" + bso["id"], bso, status=400)
    assert res.json == WEAVE_INVALID_WBO
    # Invalid sortindex - larger than max value
    bso = {"id": "TEST", "payload": "testing", "sortindex": "1" + "0" * 9}
    res = retry_post_json(app, coll_url, [bso])
    assert res.json["failed"] and not res.json["success"]
    res = retry_put_json(app, coll_url + "/" + bso["id"], bso, status=400)
    assert res.json == WEAVE_INVALID_WBO
    # Invalid payload - not a string
    bso = {"id": "TEST", "payload": 42}
    res = retry_post_json(app, coll_url, [bso])
    assert res.json["failed"] and not res.json["success"]
    res = retry_put_json(app, coll_url + "/" + bso["id"], bso, status=400)
    assert res.json == WEAVE_INVALID_WBO
    # Invalid ttl - not an integer
    bso = {"id": "TEST", "payload": "testing", "ttl": "eh?"}
    res = retry_post_json(app, coll_url, [bso])
    assert res.json["failed"] and not res.json["success"]
    res = retry_put_json(app, coll_url + "/" + bso["id"], bso, status=400)
    assert res.json == WEAVE_INVALID_WBO
    # Invalid ttl - not an integer
    bso = {"id": "TEST", "payload": "testing", "ttl": "4.2"}
    res = retry_post_json(app, coll_url, [bso])
    assert res.json["failed"] and not res.json["success"]
    res = retry_put_json(app, coll_url + "/" + bso["id"], bso, status=400)
    assert res.json == WEAVE_INVALID_WBO
    # Invalid BSO - unknown field
    bso = {"id": "TEST", "unexpected": "spanish-inquisition"}
    res = retry_post_json(app, coll_url, [bso])
    assert res.json["failed"] and not res.json["success"]
    res = retry_put_json(app, coll_url + "/" + bso["id"], bso, status=400)
    assert res.json == WEAVE_INVALID_WBO


def test_that_batch_gets_are_limited_to_max_number_of_ids(st_ctx):
    """Test that batch gets are limited to max number of ids."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    bso = {"id": "01", "payload": "testing"}
    retry_put_json(app, root + "/storage/xxx_col2/01", bso)

    # Getting with less than the limit works OK.
    ids = ",".join(str(i).zfill(2) for i in range(BATCH_MAX_IDS - 1))
    res = app.get(root + "/storage/xxx_col2?ids=" + ids)
    assert res.json == ["01"]

    # Getting with equal to the limit works OK.
    ids = ",".join(str(i).zfill(2) for i in range(BATCH_MAX_IDS))
    res = app.get(root + "/storage/xxx_col2?ids=" + ids)
    assert res.json == ["01"]

    # Getting with more than the limit fails.
    ids = ",".join(str(i).zfill(2) for i in range(BATCH_MAX_IDS + 1))
    app.get(root + "/storage/xxx_col2?ids=" + ids, status=400)


def test_that_batch_deletes_are_limited_to_max_number_of_ids(st_ctx):
    """Test that batch deletes are limited to max number of ids."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    bso = {"id": "01", "payload": "testing"}

    # Deleting with less than the limit works OK.
    retry_put_json(app, root + "/storage/xxx_col2/01", bso)
    ids = ",".join(str(i).zfill(2) for i in range(BATCH_MAX_IDS - 1))
    retry_delete(app, root + "/storage/xxx_col2?ids=" + ids)

    # Deleting with equal to the limit works OK.
    retry_put_json(app, root + "/storage/xxx_col2/01", bso)
    ids = ",".join(str(i).zfill(2) for i in range(BATCH_MAX_IDS))
    retry_delete(app, root + "/storage/xxx_col2?ids=" + ids)

    # Deleting with more than the limit fails.
    retry_put_json(app, root + "/storage/xxx_col2/01", bso)
    ids = ",".join(str(i).zfill(2) for i in range(BATCH_MAX_IDS + 1))
    retry_delete(app, root + "/storage/xxx_col2?ids=" + ids, status=400)


def test_that_expired_items_can_be_overwritten_via_PUT(st_ctx):
    """Test that expired items can be overwritten via PUT."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # Upload something with a small ttl.
    bso = {"payload": "XYZ", "ttl": 0}
    retry_put_json(app, root + "/storage/xxx_col2/TEST", bso)
    # Wait for it to expire.
    time.sleep(0.02)
    app.get(root + "/storage/xxx_col2/TEST", status=404)
    # Overwriting it should still work.
    bso = {"payload": "XYZ", "ttl": 42}
    retry_put_json(app, root + "/storage/xxx_col2/TEST", bso)


def test_if_modified_since_on_info_views(st_ctx):
    """Test if modified since on info views."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # Store something, so the views have a modified time > 0.
    bsos = [{"id": str(i).zfill(2), "payload": "xxx"} for i in range(3)]
    retry_post_json(app, root + "/storage/xxx_col1", bsos)
    INFO_VIEWS = (
        "/info/collections",
        "/info/quota",
        "/info/collection_usage",
        "/info/collection_counts",
    )
    # Get the initial last-modified version.
    r = app.get(root + "/info/collections")
    ts1 = float(r.headers["X-Last-Modified"])
    assert ts1 > 0
    # With X-I-M-S set before latest change, all should give a 200.
    headers = {"X-If-Modified-Since": str(ts1 - 1)}
    for view in INFO_VIEWS:
        app.get(root + view, headers=headers, status=200)
    # With X-I-M-S set to after latest change , all should give a 304.
    headers = {"X-If-Modified-Since": str(ts1)}
    for view in INFO_VIEWS:
        app.get(root + view, headers=headers, status=304)
    # Change a collection.
    bso = {"payload": "TEST"}
    r = retry_put_json(app, root + "/storage/xxx_col2/TEST", bso)
    ts2 = r.headers["X-Last-Modified"]
    # Using the previous version should read the updated data.
    headers = {"X-If-Modified-Since": str(ts1)}
    for view in INFO_VIEWS:
        app.get(root + view, headers=headers, status=200)
    # Using the new timestamp should produce 304s.
    headers = {"X-If-Modified-Since": str(ts2)}
    for view in INFO_VIEWS:
        app.get(root + view, headers=headers, status=304)


def test_that_x_last_modified_is_sent_for_all_get_requests(st_ctx):
    """Test that x last modified is sent for all get requests."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    bsos = [{"id": str(i).zfill(2), "payload": "xxx"} for i in range(5)]
    retry_post_json(app, root + "/storage/xxx_col2", bsos)
    r = app.get(root + "/info/collections")
    assert "X-Last-Modified" in r.headers
    r = app.get(root + "/info/collection_counts")
    assert "X-Last-Modified" in r.headers
    r = app.get(root + "/storage/xxx_col2")
    assert "X-Last-Modified" in r.headers
    r = app.get(root + "/storage/xxx_col2/01")
    assert "X-Last-Modified" in r.headers


def test_update_of_ttl_without_sending_data(st_ctx):
    """Test update of ttl without sending data."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    bso = {"payload": "x", "ttl": 1}
    retry_put_json(app, root + "/storage/xxx_col2/TEST1", bso)
    retry_put_json(app, root + "/storage/xxx_col2/TEST2", bso)
    # Before those expire, update ttl on one that exists
    # and on one that does not.
    time.sleep(0.2)
    bso = {"ttl": 10}
    retry_put_json(app, root + "/storage/xxx_col2/TEST2", bso)
    retry_put_json(app, root + "/storage/xxx_col2/TEST3", bso)
    # Update some other field on TEST1, which should leave ttl untouched.
    bso = {"sortindex": 3}
    retry_put_json(app, root + "/storage/xxx_col2/TEST1", bso)
    # If we wait, TEST1 should expire but the others should not.
    time.sleep(0.8)
    items = app.get(root + "/storage/xxx_col2?full=1").json
    items = dict((item["id"], item) for item in items)
    assert sorted(list(items.keys())) == ["TEST2", "TEST3"]
    # The existing item should have retained its payload.
    # The new item should have got a default payload of empty string.
    assert items["TEST2"]["payload"] == "x"
    assert items["TEST3"]["payload"] == ""
    ts2 = items["TEST2"]["modified"]
    ts3 = items["TEST3"]["modified"]
    assert ts2 < ts3


def test_bulk_update_of_ttls_without_sending_data(st_ctx):
    """Test bulk update of ttls without sending data."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # Create 5 BSOs with a ttl of 1 second.
    bsos = [{"id": str(i).zfill(2), "payload": "x", "ttl": 1} for i in range(5)]
    r = retry_post_json(app, root + "/storage/xxx_col2", bsos)
    ts1 = float(r.headers["X-Last-Modified"])
    # Before they expire, bulk-update the ttl to something longer.
    # Also send data for some that don't exist yet.
    # And just to be really tricky, we're also going to update
    # one of the payloads at the same time.
    time.sleep(0.2)
    bsos = [{"id": str(i).zfill(2), "ttl": 10} for i in range(3, 7)]
    bsos[0]["payload"] = "xx"
    r = retry_post_json(app, root + "/storage/xxx_col2", bsos)
    assert len(r.json["success"]) == 4
    ts2 = float(r.headers["X-Last-Modified"])
    # If we wait then items 0, 1, 2 should have expired.
    # Items 3, 4, 5, 6 should still exist.
    time.sleep(0.8)
    items = app.get(root + "/storage/xxx_col2?full=1").json
    items = dict((item["id"], item) for item in items)
    assert sorted(list(items.keys())) == ["03", "04", "05", "06"]
    # Items 3 and 4 should have the specified payloads.
    # Items 5 and 6 should have payload defaulted to empty string.
    assert items["03"]["payload"] == "xx"
    assert items["04"]["payload"] == "x"
    assert items["05"]["payload"] == ""
    assert items["06"]["payload"] == ""
    # All items created or modified by the request should get their
    # timestamps update.  Just bumping the ttl should not bump timestamp.
    assert items["03"]["modified"] == ts2
    assert items["04"]["modified"] == ts1
    assert items["05"]["modified"] == ts2
    assert items["06"]["modified"] == ts2


def test_that_negative_integer_fields_are_not_accepted(st_ctx):
    """Test that negative integer fields are not accepted."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # ttls cannot be negative
    retry_put_json(
        app,
        root + "/storage/xxx_col2/TEST",
        {
            "payload": "TEST",
            "ttl": -1,
        },
        status=400,
    )
    # limit cannot be negative
    retry_put_json(app, root + "/storage/xxx_col2/TEST", {"payload": "X"})
    app.get(root + "/storage/xxx_col2?limit=-1", status=400)
    # X-If-Modified-Since cannot be negative
    app.get(
        root + "/storage/xxx_col2",
        headers={
            "X-If-Modified-Since": "-3",
        },
        status=400,
    )
    # X-If-Unmodified-Since cannot be negative
    retry_put_json(
        app,
        root + "/storage/xxx_col2/TEST",
        {
            "payload": "TEST",
        },
        headers={
            "X-If-Unmodified-Since": "-3",
        },
        status=400,
    )
    # sortindex actually *can* be negative
    retry_put_json(
        app,
        root + "/storage/xxx_col2/TEST",
        {
            "payload": "TEST",
            "sortindex": -42,
        },
        status=200,
    )


def test_meta_global_sanity(st_ctx):
    """Test meta global sanity."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # Memcache backend is configured to store 'meta' in write-through
    # cache, so we want to check it explicitly.  We might as well put it
    # in the base tests because there's nothing memcached-specific here.
    app.get(root + "/storage/meta/global", status=404)
    res = app.get(root + "/storage/meta")
    assert res.json == []
    retry_put_json(app, root + "/storage/meta/global", {"payload": "blob"})
    res = app.get(root + "/storage/meta")
    assert res.json == ["global"]
    res = app.get(root + "/storage/meta/global")
    assert res.json["payload"] == "blob"
    # It should not have extra keys.
    keys = list(res.json.keys())
    keys.sort()
    assert keys == ["id", "modified", "payload"]
    # It should have a properly-formatted "modified" field.
    modified_re = r"['\"]modified['\"]:\s*[0-9]+\.[0-9][0-9]\s*[,}]"
    assert re.search(modified_re, res.body.decode("utf-8"))
    # Any client-specified "modified" field should be ignored
    res = retry_put_json(
        app,
        root + "/storage/meta/global",
        {"payload": "blob", "modified": 12},
    )
    ts = float(res.headers["X-Weave-Timestamp"])
    res = app.get(root + "/storage/meta/global")
    assert res.json["modified"] == ts


def test_that_404_responses_have_a_json_body(st_ctx):
    """Test that 404 responses have a json body."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    res = app.get(root + "/nonexistent/url", status=404)
    assert res.content_type == "application/json"
    assert res.json == 0


def test_that_internal_server_fields_are_not_echoed(st_ctx):
    """Test that internal server fields are not echoed."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    retry_post_json(app, root + "/storage/xxx_col1", [{"id": "one", "payload": "blob"}])
    retry_put_json(app, root + "/storage/xxx_col1/two", {"payload": "blub"})
    res = app.get(root + "/storage/xxx_col1?full=1")
    assert len(res.json) == 2
    for item in res.json:
        assert "id" in item
        assert "payload" in item
        assert "payload_size" not in item
        assert "ttl" not in item
    for id in ("one", "two"):
        res = app.get(root + "/storage/xxx_col1/" + id)
        assert "id" in res.json
        assert "payload" in res.json
        assert "payload_size" not in res.json
        assert "ttl" not in res.json


def test_accessing_info_collections_with_an_expired_token(st_ctx):
    """Test accessing info collections with an expired token."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # Write some items while we've got a good token.
    bsos = [{"id": str(i).zfill(2), "payload": "xxx"} for i in range(3)]
    resp = retry_post_json(app, root + "/storage/xxx_col1", bsos)
    ts = float(resp.headers["X-Last-Modified"])

    # Check that we can read the info correctly.
    resp = app.get(root + "/info/collections")
    assert list(resp.json.keys()) == ["xxx_col1"]
    assert resp.json["xxx_col1"] == ts

    # Forge an expired token to use for the test.
    auth_policy = st_ctx["config"].registry.getUtility(IAuthenticationPolicy)
    secret = auth_policy._get_token_secrets(st_ctx["host_url"])[-1]
    tm = tokenlib.TokenManager(secret=secret)
    exp = time.time() - 60
    data = {
        "uid": st_ctx["user_id"],
        "node": st_ctx["host_url"],
        "expires": exp,
        "hashed_fxa_uid": st_ctx["hashed_fxa_uid"],
        "fxa_uid": st_ctx["fxa_uid"],
        "fxa_kid": st_ctx["fxa_kid"],
    }
    st_ctx["auth_state"]["auth_token"] = tm.make_token(data)
    st_ctx["auth_state"]["auth_secret"] = tm.get_derived_secret(
        st_ctx["auth_state"]["auth_token"]
    )

    # The expired token cannot be used for normal operations.
    bsos = [{"id": str(i).zfill(2), "payload": "aaa"} for i in range(3)]
    retry_post_json(app, root + "/storage/xxx_col1", bsos, status=401)
    app.get(root + "/storage/xxx_col1", status=401)

    # But it still allows access to /info/collections.
    resp = app.get(root + "/info/collections")
    assert list(resp.json.keys()) == ["xxx_col1"]
    assert resp.json["xxx_col1"] == ts


def test_pagination_with_newer_and_sort_by_oldest(st_ctx):
    """Test pagination with newer and sort by oldest."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # Twelve bsos with three different modification times.
    NUM_ITEMS = 12
    bsos = []
    timestamps = []
    for i in range(NUM_ITEMS):
        bso = {"id": str(i).zfill(2), "payload": "x"}
        bsos.append(bso)
        if i % 4 == 3:
            res = retry_post_json(app, root + "/storage/xxx_col2", bsos)
            ts = float(res.headers["X-Last-Modified"])
            timestamps.append((i, ts))
            bsos = []

    # Try with several different pagination sizes,
    # to hit various boundary conditions.
    for limit in (2, 3, 4, 5, 6):
        for start, ts in timestamps:
            query_url = root + "/storage/xxx_col2?full=true&sort=oldest"
            query_url += "&newer=%s&limit=%s" % (ts, limit)

            # Paginated-ly fetch all items.
            items = []
            res = app.get(query_url)
            for item in res.json:
                if items:
                    assert items[-1]["modified"] <= item["modified"]
                items.append(item)
            next_offset = res.headers.get("X-Weave-Next-Offset")
            while next_offset is not None:
                res = app.get(query_url + "&offset=" + next_offset)
                for item in res.json:
                    assert items[-1]["modified"] <= item["modified"]
                    items.append(item)
                next_offset = res.headers.get("X-Weave-Next-Offset")

            # They should all be in order, starting from the item
            # *after* the one that was used for the newer= timestamp.
            assert sorted(int(item["id"]) for item in items) == list(
                range(start + 1, NUM_ITEMS)
            )


def test_pagination_with_older_and_sort_by_newest(st_ctx):
    """Test pagination with older and sort by newest."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # Twelve bsos with three different modification times.
    NUM_ITEMS = 12
    bsos = []
    timestamps = []
    for i in range(NUM_ITEMS):
        bso = {"id": str(i).zfill(2), "payload": "x"}
        bsos.append(bso)
        if i % 4 == 3:
            res = retry_post_json(app, root + "/storage/xxx_col2", bsos)
            ts = float(res.headers["X-Last-Modified"])
            timestamps.append((i - 3, ts))
            bsos = []

    # Try with several different pagination sizes,
    # to hit various boundary conditions.
    for limit in (2, 3, 4, 5, 6):
        for start, ts in timestamps:
            query_url = root + "/storage/xxx_col2?full=true&sort=newest"
            query_url += "&older=%s&limit=%s" % (ts, limit)

            # Paginated-ly fetch all items.
            items = []
            res = app.get(query_url)
            for item in res.json:
                if items:
                    assert items[-1]["modified"] >= item["modified"]
                items.append(item)
            next_offset = res.headers.get("X-Weave-Next-Offset")
            while next_offset is not None:
                res = app.get(query_url + "&offset=" + next_offset)
                for item in res.json:
                    assert items[-1]["modified"] >= item["modified"]
                    items.append(item)
                next_offset = res.headers.get("X-Weave-Next-Offset")

            # They should all be in order, up to the item *before*
            # the one that was used for the older= timestamp.
            assert sorted(int(item["id"]) for item in items) == list(range(0, start))


def test_batches(st_ctx):
    """Test batches."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    endpoint = root + "/storage/xxx_col2"

    bso1 = {"id": "12", "payload": "elegance"}
    bso2 = {"id": "13", "payload": "slovenly"}
    bsos = [bso1, bso2]
    retry_post_json(app, endpoint, bsos)

    resp = app.get(endpoint + "/12")
    orig_modified = resp.headers["X-Last-Modified"]

    bso3 = {"id": "a", "payload": "internal"}
    bso4 = {"id": "b", "payload": "pancreas"}
    resp = retry_post_json(app, endpoint + "?batch=true", [bso3, bso4])
    batch = resp.json["batch"]

    # The collection should not be reported as modified.
    assert orig_modified == resp.headers["X-Last-Modified"]

    # And reading from it shouldn't show the new records yet.
    resp = app.get(endpoint)
    res = resp.json
    res.sort()
    assert res == ["12", "13"]
    assert int(resp.headers["X-Weave-Records"]) == 2
    assert orig_modified == resp.headers["X-Last-Modified"]

    bso5 = {"id": "c", "payload": "tinsel"}
    bso6 = {"id": "13", "payload": "portnoy"}
    bso0 = {"id": "14", "payload": "itsybitsy"}
    commit = "?batch={0}&commit=true".format(batch)
    resp = retry_post_json(app, endpoint + commit, [bso5, bso6, bso0])
    committed = resp.json["modified"]
    print(committed)
    assert resp.json["modified"] == float(resp.headers["X-Last-Modified"])

    # make sure /info/collections got updated
    resp = app.get(root + "/info/collections")
    assert float(resp.headers["X-Last-Modified"]) == committed
    assert resp.json["xxx_col2"] == committed

    # make sure the changes applied
    resp = app.get(endpoint)
    res = resp.json
    res.sort()
    assert res == ["12", "13", "14", "a", "b", "c"]
    assert int(resp.headers["X-Weave-Records"]) == 6
    resp = app.get(endpoint + "/13")
    assert resp.json["payload"] == "portnoy"
    assert committed == float(resp.headers["X-Last-Modified"])
    assert committed == resp.json["modified"]
    resp = app.get(endpoint + "/c")
    assert resp.json["payload"] == "tinsel"
    assert committed == resp.json["modified"]
    resp = app.get(endpoint + "/14")
    assert resp.json["payload"] == "itsybitsy"
    assert committed == resp.json["modified"]

    # empty commit POST
    bso7 = {"id": "a", "payload": "burrito"}
    bso8 = {"id": "e", "payload": "chocolate"}
    resp = retry_post_json(app, endpoint + "?batch=true", [bso7, bso8])
    batch = resp.json["batch"]
    time.sleep(1)
    commit = "?batch={0}&commit=true".format(batch)

    resp1 = retry_post_json(app, endpoint + commit, [])
    committed = resp1.json["modified"]
    assert committed == float(resp1.headers["X-Last-Modified"])

    resp2 = app.get(endpoint + "/a")
    assert committed == float(resp2.headers["X-Last-Modified"])
    assert committed == resp2.json["modified"]
    assert resp2.json["payload"] == "burrito"

    resp3 = app.get(endpoint + "/e")
    assert committed == resp3.json["modified"]


def test_aaa_batch_commit_collision(st_ctx):
    """Test aaa batch commit collision."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # It's possible that a batch contain a BSO inside a batch as well
    # as inside the final "commit" message. This is a bit of a problem
    # for spanner because of conflicting ways that the data is written
    # to the database and the discoverability of IDs in previously
    # submitted batches.
    endpoint = root + "/storage/xxx_col2"
    orig = "Letting the days go by"
    repl = "Same as it ever was"

    batch_num = retry_post_json(
        app, endpoint + "?batch=true", [{"id": "b0", "payload": orig}]
    ).json["batch"]

    resp = retry_post_json(
        app,
        endpoint + "?batch={}&commit=true".format(batch_num),
        [{"id": "b0", "payload": repl}],
    )

    # this should succeed, using the newerer payload value.
    assert resp.json["failed"] == {}, "batch commit failed"
    assert resp.json["success"] == ["b0"], "batch commit id incorrect"
    resp = app.get(endpoint + "?full=1")
    assert resp.json[0].get("payload") == repl, "wrong payload returned"


def test_we_dont_need_no_stinkin_batches(st_ctx):
    """Test we dont need no stinkin batches."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    endpoint = root + "/storage/xxx_col2"

    # invalid batch ID
    bso1 = {"id": "f", "payload": "pantomime"}
    retry_post_json(app, endpoint + "?batch=sammich", [bso1], status=400)

    # commit with no batch ID
    retry_post_json(app, endpoint + "?commit=true", [], status=400)


def test_batch_size_limits(st_ctx):
    """Test batch size limits."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    limits = app.get(root + "/info/configuration").json
    assert "max_post_records" in limits
    assert "max_post_bytes" in limits
    assert "max_total_records" in limits
    assert "max_total_bytes" in limits
    assert "max_record_payload_bytes" in limits
    assert "max_request_bytes" in limits

    endpoint = root + "/storage/xxx_col2?batch=true"
    bsos = [
        {"id": "little", "payload": "XXX"},
        {"id": "big", "payload": "X" * (limits["max_post_bytes"] - 3)},
    ]
    res = retry_post_json(app, endpoint, bsos)
    assert not res.json["failed"]
    bsos[1]["payload"] += "X"
    res = retry_post_json(app, endpoint, bsos)
    assert res.json["success"] == ["little"]
    assert res.json["failed"]["big"] == "retry bytes"

    # `max_total_bytes` is an (inclusive) limit on the
    # total size of all payloads in a batch.  We can only enforce
    # it if the client tells us this via header.

    retry_post_json(
        app,
        endpoint,
        [],
        headers={"X-Weave-Total-Bytes": str(limits["max_total_bytes"])},
    )
    res = retry_post_json(
        app,
        endpoint,
        [],
        headers={"X-Weave-Total-Bytes": str(limits["max_total_bytes"] + 1)},
        status=400,
    )
    assert res.json == WEAVE_SIZE_LIMIT_EXCEEDED


def test_batch_partial_update(st_ctx):
    """Test batch partial update."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    collection = root + "/storage/xxx_col2"
    bsos = [
        {"id": "a", "payload": "aai"},
        {"id": "b", "payload": "bee", "sortindex": 17},
    ]
    resp = retry_post_json(app, collection, bsos)
    orig_ts = float(resp.headers["X-Last-Modified"])

    # Update one, and add a new one.
    bsos = [
        {"id": "b", "payload": "bii"},
        {"id": "c", "payload": "sea"},
    ]
    resp = retry_post_json(app, collection + "?batch=true", bsos)
    batch = resp.json["batch"]
    assert orig_ts == float(resp.headers["X-Last-Modified"])

    # The updated item hasn't been written yet.
    resp = app.get(collection + "?full=1")
    res = resp.json
    res.sort(key=lambda bso: bso["id"])
    assert len(res) == 2
    assert res[0]["payload"] == "aai"
    assert res[1]["payload"] == "bee"
    assert res[0]["modified"] == orig_ts
    assert res[1]["modified"] == orig_ts
    assert res[1]["sortindex"] == 17

    endpoint = collection + "?batch={0}&commit=true".format(batch)
    resp = retry_post_json(app, endpoint, [])
    commit_ts = float(resp.headers["X-Last-Modified"])

    # The changes have now been applied.
    resp = app.get(collection + "?full=1")
    res = resp.json
    res.sort(key=lambda bso: bso["id"])
    assert len(res) == 3
    assert res[0]["payload"] == "aai"
    assert res[1]["payload"] == "bii"
    assert res[2]["payload"] == "sea"
    assert res[0]["modified"] == orig_ts
    assert res[1]["modified"] == commit_ts
    assert res[2]["modified"] == commit_ts

    # Fields not touched by the batch, should have been preserved.
    assert res[1]["sortindex"] == 17


def test_batch_ttl_update(st_ctx):
    """Test batch ttl update."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    collection = root + "/storage/xxx_col2"
    bsos = [
        {"id": "a", "payload": "ayy"},
        {"id": "b", "payload": "bea"},
        {"id": "c", "payload": "see"},
    ]
    retry_post_json(app, collection, bsos)

    # Bump ttls as a series of individual batch operations.
    resp = retry_post_json(app, collection + "?batch=true", [], status=202)
    orig_ts = float(resp.headers["X-Last-Modified"])
    batch = resp.json["batch"]

    endpoint = collection + "?batch={0}".format(batch)
    resp = retry_post_json(app, endpoint, [{"id": "a", "ttl": 2}], status=202)
    assert orig_ts == float(resp.headers["X-Last-Modified"])
    resp = retry_post_json(app, endpoint, [{"id": "b", "ttl": 2}], status=202)
    assert orig_ts == float(resp.headers["X-Last-Modified"])
    retry_post_json(app, endpoint + "&commit=true", [], status=200)

    # The payloads should be unchanged
    resp = app.get(collection + "?full=1")
    res = resp.json
    res.sort(key=lambda bso: bso["id"])
    assert len(res) == 3
    assert res[0]["payload"] == "ayy"
    assert res[1]["payload"] == "bea"
    assert res[2]["payload"] == "see"

    # If we wait, the ttls should kick in
    time.sleep(2.1)
    resp = app.get(collection + "?full=1")
    res = resp.json
    assert len(res) == 1
    assert res[0]["payload"] == "see"


def test_batch_ttl_is_based_on_commit_timestamp(st_ctx):
    """Test batch ttl is based on commit timestamp."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    collection = root + "/storage/xxx_col2"

    resp = retry_post_json(app, collection + "?batch=true", [], status=202)
    batch = resp.json["batch"]
    endpoint = collection + "?batch={0}".format(batch)
    retry_post_json(app, endpoint, [{"id": "a", "ttl": 3}], status=202)

    # Put some time between upload timestamp and commit timestamp.
    time.sleep(1.5)

    retry_post_json(app, endpoint + "&commit=true", [], status=200)

    # Wait a little; if ttl is taken from the time of the commit
    # then it should not kick in just yet.
    time.sleep(1.6)
    resp = app.get(collection)
    res = resp.json
    assert len(res) == 1
    assert res[0] == "a"

    # Wait some more, and the ttl should kick in.
    time.sleep(1.6)
    resp = app.get(collection)
    res = resp.json
    assert len(res) == 0


def test_batch_with_immediate_commit(st_ctx):
    """Test batch with immediate commit."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    collection = root + "/storage/xxx_col2"
    bsos = [
        {"id": "a", "payload": "aih"},
        {"id": "b", "payload": "bie"},
        {"id": "c", "payload": "cee"},
    ]

    resp = retry_post_json(
        app, collection + "?batch=true&commit=true", bsos, status=200
    )
    assert "batch" not in resp.json
    assert "modified" in resp.json
    committed = resp.json["modified"]

    resp = app.get(root + "/info/collections")
    assert float(resp.headers["X-Last-Modified"]) == committed
    assert resp.json["xxx_col2"] == committed

    resp = app.get(collection + "?full=1")
    assert float(resp.headers["X-Last-Modified"]) == committed
    res = resp.json
    res.sort(key=lambda bso: bso["id"])
    assert len(res) == 3
    assert res[0]["payload"] == "aih"
    assert res[1]["payload"] == "bie"
    assert res[2]["payload"] == "cee"


def test_batch_uploads_properly_update_info_collections(st_ctx):
    """Test batch uploads properly update info collections."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    collection1 = root + "/storage/xxx_col1"
    collection2 = root + "/storage/xxx_col2"
    bsos = [
        {"id": "a", "payload": "aih"},
        {"id": "b", "payload": "bie"},
        {"id": "c", "payload": "cee"},
    ]

    resp = retry_post_json(app, collection1, bsos)
    ts1 = resp.json["modified"]

    resp = retry_post_json(app, collection2, bsos)
    ts2 = resp.json["modified"]

    resp = app.get(root + "/info/collections")
    assert float(resp.headers["X-Last-Modified"]) == ts2
    assert resp.json["xxx_col1"] == ts1
    assert resp.json["xxx_col2"] == ts2

    # Overwrite in place, timestamp should change.
    resp = retry_post_json(app, collection2 + "?batch=true&commit=true", bsos[:2])
    assert resp.json["modified"] > ts2
    ts2 = resp.json["modified"]

    resp = app.get(root + "/info/collections")
    assert float(resp.headers["X-Last-Modified"]) == ts2
    assert resp.json["xxx_col1"] == ts1
    assert resp.json["xxx_col2"] == ts2

    # Add new items, timestamp should change
    resp = retry_post_json(
        app,
        collection1 + "?batch=true&commit=true",
        [{"id": "d", "payload": "dee"}],
    )
    assert resp.json["modified"] > ts1
    assert resp.json["modified"] >= ts2
    ts1 = resp.json["modified"]

    resp = app.get(root + "/info/collections")
    assert float(resp.headers["X-Last-Modified"]) == ts1
    assert resp.json["xxx_col1"] == ts1
    assert resp.json["xxx_col2"] == ts2


def test_batch_with_failing_bsos(st_ctx):
    """Test batch with failing bsos."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    collection = root + "/storage/xxx_col2"
    bsos = [
        {"id": "a", "payload": "aai"},
        {"id": "b\n", "payload": "i am invalid", "sortindex": 17},
    ]
    resp = retry_post_json(app, collection + "?batch=true", bsos)
    assert len(resp.json["failed"]) == 1
    assert len(resp.json["success"]) == 1
    batch = resp.json["batch"]

    bsos = [
        {"id": "c", "payload": "sea"},
        {"id": "d", "payload": "dii", "ttl": -12},
    ]
    endpoint = collection + "?batch={0}&commit=true".format(batch)
    resp = retry_post_json(app, endpoint, bsos)
    assert len(resp.json["failed"]) == 1
    assert len(resp.json["success"]) == 1

    # To correctly match semantics of batchless POST, the batch
    # should be committed including only the successful items.
    # It is the client's responsibility to detect that some items
    # failed, and decide whether to commit the batch.
    resp = app.get(collection + "?full=1")
    res = resp.json
    res.sort(key=lambda bso: bso["id"])
    assert len(res) == 2
    assert res[0]["payload"] == "aai"
    assert res[1]["payload"] == "sea"


def test_batch_id_is_correctly_scoped_to_a_collection(st_ctx):
    """Test batch id is correctly scoped to a collection."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    collection1 = root + "/storage/xxx_col1"
    bsos = [
        {"id": "a", "payload": "aih"},
        {"id": "b", "payload": "bie"},
        {"id": "c", "payload": "cee"},
    ]
    resp = retry_post_json(app, collection1 + "?batch=true", bsos)
    batch = resp.json["batch"]

    # I should not be able to add to that batch in a different collection.
    endpoint2 = root + "/storage/xxx_col2?batch={0}".format(batch)
    retry_post_json(app, endpoint2, [{"id": "d", "payload": "dii"}], status=400)

    # I should not be able to commit that batch in a different collection.
    retry_post_json(app, endpoint2 + "&commit=true", [], status=400)

    # I should still be able to use the batch in the correct collection.
    endpoint1 = collection1 + "?batch={0}".format(batch)
    retry_post_json(app, endpoint1, [{"id": "d", "payload": "dii"}])
    retry_post_json(app, endpoint1 + "&commit=true", [])

    resp = app.get(collection1 + "?full=1")
    res = resp.json
    res.sort(key=lambda bso: bso["id"])
    assert len(res) == 4
    assert res[0]["payload"] == "aih"
    assert res[1]["payload"] == "bie"
    assert res[2]["payload"] == "cee"
    assert res[3]["payload"] == "dii"


def test_users_with_the_same_batch_id_get_separate_data(st_ctx):
    """Test users with the same batch id get separate data."""
    app = st_ctx["app"]
    # Try to generate two users with the same batch-id.
    # It might take a couple of attempts...
    for _ in range(100):
        bsos = [{"id": "a", "payload": "aih"}]
        req = "/storage/xxx_col1?batch=true"
        resp = retry_post_json(app, st_ctx["root"] + req, bsos)
        batch1 = resp.json["batch"]
        with switch_user(st_ctx):
            bsos = [{"id": "b", "payload": "bee"}]
            req = "/storage/xxx_col1?batch=true"
            resp = retry_post_json(app, st_ctx["root"] + req, bsos)
            batch2 = resp.json["batch"]
            # Let the second user commit their batch.
            req = "/storage/xxx_col1?batch={0}&commit=true".format(batch2)
            retry_post_json(app, st_ctx["root"] + req, [])
            # It should only have a single item.
            resp = app.get(st_ctx["root"] + "/storage/xxx_col1")
            assert resp.json == ["b"]
        # The first user's collection should still be empty.
        # Now have the  first user commit their batch.
        req = "/storage/xxx_col1?batch={0}&commit=true".format(batch1)
        retry_post_json(app, st_ctx["root"] + req, [])
        # It should only have a single item.
        resp = app.get(st_ctx["root"] + "/storage/xxx_col1")
        assert resp.json == ["a"]
        # If we didn't make a conflict, try again.
        if batch1 == batch2:
            break
    else:
        pytest.skip("failed to generate conflicting batchid")


def test_that_we_dont_resurrect_committed_batches(st_ctx):
    """Test that we dont resurrect committed batches."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    # This retry loop tries to trigger a situation where we:
    #  * create a batch with a single item
    #  * successfully commit that batch
    #  * create a new batch tht re-uses the same batchid
    for _ in range(100):
        bsos = [{"id": "i", "payload": "aye"}]
        req = "/storage/xxx_col1?batch=true"
        resp = retry_post_json(app, root + req, bsos)
        batch1 = resp.json["batch"]
        req = "/storage/xxx_col1?batch={0}&commit=true".format(batch1)
        retry_post_json(app, root + req, [])
        req = "/storage/xxx_col2?batch=true"
        resp = retry_post_json(app, root + req, [])
        batch2 = resp.json["batch"]
        bsos = [{"id": "j", "payload": "jay"}]
        req = "/storage/xxx_col2?batch={0}&commit=true".format(batch2)
        retry_post_json(app, root + req, bsos)
        # Retry if we failed to trigger re-use of the batchid.
        if batch1 == batch2:
            break
    else:
        pytest.skip("failed to trigger re-use of batchid")

    # Despite having the same batchid, the second batch should
    # be completely independent of the first.
    resp = app.get(root + "/storage/xxx_col2")
    assert resp.json == ["j"]


def test_batch_id_is_correctly_scoped_to_a_user(st_ctx):
    """Test batch id is correctly scoped to a user."""
    app = st_ctx["app"]
    collection = st_ctx["root"] + "/storage/xxx_col1"
    bsos = [
        {"id": "a", "payload": "aih"},
        {"id": "b", "payload": "bie"},
        {"id": "c", "payload": "cee"},
    ]
    resp = retry_post_json(app, collection + "?batch=true", bsos)
    batch = resp.json["batch"]

    with switch_user(st_ctx):
        # I should not be able to add to that batch as a different user.
        endpoint = st_ctx["root"] + "/storage/xxx_col1?batch={0}".format(batch)
        retry_post_json(app, endpoint, [{"id": "d", "payload": "di"}], status=400)

        # I should not be able to commit that batch as a different user.
        retry_post_json(app, endpoint + "&commit=true", [], status=400)

    # I should still be able to use the batch in the original user.
    endpoint = collection + "?batch={0}".format(batch)
    retry_post_json(app, endpoint, [{"id": "d", "payload": "di"}])
    retry_post_json(app, endpoint + "&commit=true", [])

    resp = app.get(collection + "?full=1")
    res = resp.json
    res.sort(key=lambda bso: bso["id"])
    assert len(res) == 4
    assert res[0]["payload"] == "aih"
    assert res[1]["payload"] == "bie"
    assert res[2]["payload"] == "cee"
    assert res[3]["payload"] == "di"


# bug 1332552 make sure ttl:null use the default ttl
def test_create_bso_with_null_ttl(st_ctx):
    """Test create bso with null ttl."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    bso = {"payload": "x", "ttl": None}
    retry_put_json(app, root + "/storage/xxx_col2/TEST1", bso)
    time.sleep(0.1)
    res = app.get(root + "/storage/xxx_col2/TEST1?full=1")
    assert res.json["payload"] == "x"


def test_rejection_of_known_bad_payloads(st_ctx):
    """Test rejection of known bad payloads."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    bso = {
        "id": "keys",
        "payload": json_dumps(
            {
                "ciphertext": "IDontKnowWhatImDoing",
                "IV": "AAAAAAAAAAAAAAAAAAAAAA==",
            }
        ),
    }
    # Fishy IVs are rejected on the "crypto" collection.
    retry_put_json(app, root + "/storage/crypto/keys", bso, status=400)
    retry_put_json(app, root + "/storage/crypto/blerg", bso, status=400)
    retry_post_json(app, root + "/storage/crypto", [bso], status=400)
    # But are allowed on other collections.
    retry_put_json(app, root + "/storage/xxx_col2/keys", bso, status=200)
    retry_post_json(app, root + "/storage/xxx_col2", [bso], status=200)


# bug 1397357
def test_batch_empty_commit(st_ctx):
    """Test batch empty commit."""
    app = st_ctx["app"]
    root = st_ctx["root"]

    def testEmptyCommit(contentType, body, status=200):
        bsos = [{"id": str(i).zfill(2), "payload": "X"} for i in range(5)]
        res = retry_post_json(app, root + "/storage/xxx_col?batch=true", bsos)
        assert len(res.json["success"]) == 5
        assert len(res.json["failed"]) == 0
        batch = res.json["batch"]
        app.post(
            root + "/storage/xxx_col?commit=true&batch=" + batch,
            body,
            headers={"Content-Type": contentType},
            status=status,
        )

    testEmptyCommit("application/json", "[]")
    testEmptyCommit("application/json", "{}", status=400)
    testEmptyCommit("application/json", "", status=400)

    testEmptyCommit("application/newlines", "")
    testEmptyCommit("application/newlines", "\n", status=400)
    testEmptyCommit("application/newlines", "{}", status=400)
    testEmptyCommit("application/newlines", "[]", status=400)


def test_cors_settings_are_set(st_ctx):
    """Test cors settings are set."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    res = app.options(
        root + "/__heartbeat__",
        headers={
            "Access-Control-Request-Method": "GET",
            "Origin": "localhost",
            "Access-Control-Request-Headers": "Content-Type",
        },
    )

    assert int(res.headers["access-control-max-age"]) > 0
    assert res.headers["access-control-allow-origin"] == "localhost"


def test_cors_allows_any_origin(st_ctx):
    """Test cors allows any origin."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    app.options(
        root + "/__heartbeat__",
        headers={
            "Access-Control-Request-Method": "GET",
            "Origin": "http://test-website.com",
            "Access-Control-Request-Headers": "Content-Type",
        },
        status=[200, 204],
    )


# PATCH is not a default allowed method, so request should return 405
def test_patch_is_not_allowed(st_ctx):
    """Test patch is not allowed."""
    app = st_ctx["app"]
    root = st_ctx["root"]
    collection = root + "/storage/xxx_col1"
    with pytest.raises(AppError) as exc_info:
        app.patch_json(collection)
    assert "405" in str(exc_info.value)
