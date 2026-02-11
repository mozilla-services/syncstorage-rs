"""Load testing scenarios for SyncStorage using Molotov."""

import sys

sys.path.append(".")  # NOQA
import base64
import json
import os
import random
import time

from molotov import scenario, setup_session, teardown_session
from storage import StorageClient

_PAYLOAD = """\
This is the metaglobal payload which contains
some client data that doesnt look much
like this
"""
_WEIGHTS = {
    "metaglobal": [40, 60, 0, 0, 0],
    "distribution": [80, 15, 4, 1],
    "count_distribution": [71, 15, 7, 4, 3],
    "post_count_distribution": [67, 18, 9, 4, 2],
    "delete_count_distribution": [99, 1, 0, 0, 0],
}

_PROBS = {"get": 0.1, "post": 0.2, "deleteall": 0.01}
_COLLS = ["bookmarks", "forms", "passwords", "history", "prefs"]
_BATCH_MAX_COUNT = 100

_DISABLE_DELETES = os.environ.get("DISABLE_DELETES", "false").lower() in ("true", "1")


def should_do(name):
    """Determine if an action should be performed based on probability.

    Args:
        name: The name of the action to check.

    Returns:
        bool: True if the action should be performed, False otherwise.

    """
    return random.random() <= _PROBS[name]


def get_num_requests(name):
    """Get the number of requests to make based on weighted distribution.

    Args:
        name: The name of the request type to get count for.

    Returns:
        int: The number of requests to make.

    """
    weights = _WEIGHTS[name]
    i = random.randint(1, sum(weights))
    count = 0
    base = 0
    for weight in weights:
        base += weight
        if i <= base:
            break
        count += 1
    return count


@setup_session()
async def _session(worker_num, session):
    exc = []

    def _run():
        try:
            session.storage = StorageClient(session)
        except Exception as e:
            exc.append(e)

    # XXX code will be migrated to Molotov
    # see https://github.com/loads/molotov/issues/100
    import threading

    t = threading.Thread(target=_run)
    t.start()
    t.join()
    if len(exc) > 0:
        raise exc[0]


@teardown_session()
async def _teardown_session(worker_num, session):
    if hasattr(session, "storage") and session.storage:
        session.storage.cleanup()


@scenario(1)
async def test(session):
    """Main load test scenario for SyncStorage.

    Args:
        session: The Molotov session object.

    """
    storage = session.storage

    # Respect the server limits.
    _, config = await storage.get("/info/configuration")
    # print("Config: {}".format(json.dumps(config, indent=3)))
    # fix up consts
    payload = _PAYLOAD[: config.get("max_record_payload_bytes")]
    # GET requests to meta/global
    num_requests = min(get_num_requests("metaglobal"), config.get("max_post_records"))
    batch_max_count = min(_BATCH_MAX_COUNT, config.get("max_total_records"))

    # Always GET info/collections
    # This is also a good opportunity to correct for timeskew.
    url = "/info/collections"
    resp, _ = await storage.get(url, (200, 404))

    url = "/storage/meta/global"

    for x in range(num_requests):
        resp, __ = await storage.get(url, (200, 404))
        if resp.status == 404:
            data = json.dumps({"id": "global", "payload": payload})
            await storage.put(url, data=data, statuses=(200,))

    # Occasional reads of client records.
    if should_do("get"):
        url = "/storage/clients"
        newer = int(time.time() - random.randint(3600, 360000))
        params = {"full": "1", "newer": str(newer)}
        await storage.get(url, params=params, statuses=(200, 404))

    # Occasional updates to client records.
    if should_do("post"):
        cid = str(get_num_requests("distribution"))
        url = "/storage/clients"
        wbo = {"id": "client" + cid, "payload": cid * 300}
        data = json.dumps([wbo])
        resp, result = await storage.post(url, data=data, statuses=(200,))
        assert len(result["success"]) == 1, "No success records"
        assert len(result["failed"]) == 0, "Found failed record"

    # GET requests to individual collections.
    num_requests = get_num_requests("count_distribution")
    cols = random.sample(_COLLS, num_requests)
    for x in range(num_requests):
        url = "/storage/" + cols[x]
        newer = int(time.time() - random.randint(3600, 360000))
        params = {"full": "1", "newer": str(newer)}
        await storage.get(url, params=params, statuses=(200, 404))

    # POST requests with several WBOs batched together
    num_requests = get_num_requests("post_count_distribution")
    # Let's do roughly 50% transactional batches.
    transact = random.randint(0, 1)
    batch_id = None
    committing = False

    # Collections should be a single static entry if we're "transactional"
    if transact:
        col = random.sample(_COLLS, 1)[0]
        cols = [col for x in range(num_requests)]
    else:
        cols = random.sample(_COLLS, num_requests)

    for x in range(num_requests):
        url = "/storage/" + cols[x]
        data = []
        # Random batch size, skewed slightly towards the upper limit.
        items_per_batch = min(random.randint(20, batch_max_count + 80), batch_max_count)
        for _i in range(items_per_batch):
            randomness = os.urandom(10)
            id = base64.urlsafe_b64encode(randomness).rstrip(b"=")
            id = id.decode("utf8")
            id += str(int((time.time() % 100) * 100000))
            # Random payload length.  They can be big, but skew small.
            # This gives min=300, mean=450, max=config.max_record_payload_bytes
            payload_length = min(
                int(random.paretovariate(3) * 300),
                config.get("max_record_payload_bytes"),
            )

            # XXX should be in the class
            token = storage.auth_token.decode("utf8")
            payload_chunks = int((payload_length / len(token)) + 1)
            payload = (token * payload_chunks)[:payload_length]
            wbo = {"id": id, "payload": payload}
            data.append(wbo)

        data = json.dumps(data)
        status = 200
        if transact:
            # Batch uploads only return a 200 on commit.  An Accepted(202)
            # is returned for batch creation & appends
            status = 202
            if x == 0:
                committing = False
                url += "?batch=true"
            elif x == num_requests - 1:
                url += "?commit=true&batch=%s" % batch_id
                committing = True
                batch_id = None
                status = 200
            else:
                url += "?batch=%s" % batch_id

        resp, result = await storage.post(url, data=data, statuses=(status,))
        assert len(result["success"]) == items_per_batch, (
            "Result success did not have expected number ofitems in batch {}".format(
                result
            )
        )
        assert len(result["failed"]) == 0, "Result contained failed records: {}".format(
            result
        )

        if transact and not committing:
            batch_id = result["batch"]

    if not _DISABLE_DELETES:
        # DELETE requests.
        # We might choose to delete some individual collections, or to
        # do a full reset and delete all the data.  Never both in the
        # same run.
        num_requests = get_num_requests("delete_count_distribution")
        if num_requests:
            cols = random.sample(_COLLS, num_requests)
            for x in range(num_requests):
                url = "/storage/" + cols[x]
                resp, result = await storage.delete(url, statuses=(200, 204))
        else:
            if should_do("deleteall"):
                url = "/storage"
                resp, result = await storage.delete(url, statuses=(200,))
