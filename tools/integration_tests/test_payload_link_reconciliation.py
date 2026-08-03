# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""End-to-end tests for the payload-link reconciliation pipeline.

Requires the docker-compose.e2e.reconciliation.yaml stack: syncserver
with GCS offload enabled, fake-gcs-server, Pub/Sub emulator, the
payload-link publisher (Python by default, optionally Java), and the
reconciler running in long-running mode.

These tests self-skip when ``GCS_PAYLOAD_BUCKET`` is unset in the
environment -- so the same file lives happily under
``tools/integration_tests/`` and gets picked up by the existing
spanner e2e suite without failing (skipped entries are visible signal,
not noise). The reconciliation compose overlay sets that env var on
the ``e2e-tests`` container so the tests un-skip there.
"""

import os
import time
from typing import Any, Callable, TypeVar

import pytest
from google.cloud import storage

pytestmark = pytest.mark.skipif(
    not os.environ.get("GCS_PAYLOAD_BUCKET"),
    reason="requires the reconciliation compose stack (GCS_PAYLOAD_BUCKET env var)",
)


PAYLOAD_BUCKET = os.environ.get("GCS_PAYLOAD_BUCKET", "")
POLL_TIMEOUT_SECONDS = int(os.environ.get("RECONCILER_POLL_TIMEOUT", "30"))
POLL_INTERVAL_SECONDS = 0.5

# Standard sync collection also present in this stack's offload list.
OFFLOAD_COLLECTION = "crypto"

# Large enough that the offload code path is exercised regardless of
# any future size-threshold gate on the write side.
LARGE_PAYLOAD = "x" * 300_000


T = TypeVar("T")


@pytest.fixture(scope="module")
def gcs() -> storage.Client:
    """Storage client pointed at fake-gcs-server via STORAGE_EMULATOR_HOST."""
    return storage.Client()


def _wait_for(predicate: Callable[[], T], description: str) -> T:
    """Poll ``predicate`` until truthy or timeout; return its final value."""
    deadline = time.monotonic() + POLL_TIMEOUT_SECONDS
    last: T | None = None
    while time.monotonic() < deadline:
        last = predicate()
        if last:
            return last
        time.sleep(POLL_INTERVAL_SECONDS)
    pytest.fail(f"timeout waiting for: {description}")


def _put_bso(st_ctx: dict[str, Any], bso_id: str, payload: str = LARGE_PAYLOAD) -> None:
    from tools.integration_tests.helpers import retry_put_json

    retry_put_json(
        st_ctx["app"],
        f"{st_ctx['root']}/storage/{OFFLOAD_COLLECTION}/{bso_id}",
        {"payload": payload, "sortindex": 1},
    )


def _list_blobs(gcs: storage.Client, prefix: str) -> list[storage.Blob]:
    return list(gcs.bucket(PAYLOAD_BUCKET).list_blobs(prefix=prefix))


def _prefix_for(st_ctx: dict[str, Any], bso_id: str) -> str:
    return f"{st_ctx['fxa_uid']}/{OFFLOAD_COLLECTION}/{bso_id}/"


def test_upload_finalizes_object(st_ctx: dict[str, Any], gcs: storage.Client) -> None:
    """A newly-offloaded object gets committed=true + customTime=MAX."""
    bso_id = "reconcile-upload"
    _put_bso(st_ctx, bso_id)

    def committed_blob() -> storage.Blob | None:
        for blob in _list_blobs(gcs, _prefix_for(st_ctx, bso_id)):
            blob.reload()
            if (blob.metadata or {}).get("committed") == "true":
                return blob
        return None

    blob = _wait_for(committed_blob, "committed=true on new object")
    assert blob is not None
    # customTime should be pinned to the far-future sentinel.
    assert blob.custom_time is not None
    assert blob.custom_time.year == 9999


def test_update_deletes_old_object(st_ctx: dict[str, Any], gcs: storage.Client) -> None:
    """Replacing payload_link deletes the prior object."""
    bso_id = "reconcile-update"
    _put_bso(st_ctx, bso_id, payload="a" * 300_000)
    old = _wait_for(
        lambda: (_list_blobs(gcs, _prefix_for(st_ctx, bso_id)) or [None])[0],
        "initial object exists",
    )
    assert old is not None

    # Rewrite -- publisher path -> reconciler -> old object deleted, new committed.
    _put_bso(st_ctx, bso_id, payload="b" * 300_000)

    _wait_for(
        lambda: not gcs.bucket(PAYLOAD_BUCKET).blob(old.name).exists(),
        "old object deleted",
    )


def test_delete_removes_object(st_ctx: dict[str, Any], gcs: storage.Client) -> None:
    """Deleting the BSO row deletes its GCS object."""
    bso_id = "reconcile-delete"
    _put_bso(st_ctx, bso_id)
    blob = _wait_for(
        lambda: (_list_blobs(gcs, _prefix_for(st_ctx, bso_id)) or [None])[0],
        "object exists after upload",
    )
    assert blob is not None

    st_ctx["app"].delete(f"{st_ctx['root']}/storage/{OFFLOAD_COLLECTION}/{bso_id}")

    _wait_for(
        lambda: not gcs.bucket(PAYLOAD_BUCKET).blob(blob.name).exists(),
        "object deleted after row delete",
    )
