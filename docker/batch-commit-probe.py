#!/usr/bin/env python3
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# Diagnostic for the batch-commit payload handoff (STOR-657 / STOR-668).
#
# The load test showed a large payload_reconciler.gcs_404{op=finalize} count
# and zero batch_commit_skips, which is the signature of the reconciler
# deleting a GCS object that a committed bsos row still points at. This isolates
# that path from the rest of the load: one transactional batch, one commit, then
# read the BSO back.
#
# Run inside the rig's network, from the loadtest image (it already has
# tokenlib and requests):
#
#   docker compose ... run --rm --entrypoint python3 \
#     -v "$PWD/docker/batch-commit-probe.py:/probe.py:ro" loadtest /probe.py
#
# Exits 0 if the payload survives the commit, 1 if it does not.

import hashlib
import hmac
import json
import os
import random
import sys
import time
from base64 import b64encode
from urllib.parse import urlparse

import requests
from tokenlib import get_derived_secret as derive
from tokenlib import make_token

SERVER_URL = os.environ.get("SERVER_URL", "http://syncserver:8000#secret0")
# Must be in the server's gcs_payload_offload_collections.
COLLECTION = os.environ.get("PROBE_COLLECTION", "bookmarks")
# Comfortably over any inline threshold, well under the 2.5 MiB Spanner cap so
# the probe also works against a stock server.
PAYLOAD_SIZE = int(os.environ.get("PROBE_PAYLOAD_SIZE", "300000"))
SETTLE_SECONDS = int(os.environ.get("PROBE_SETTLE_SECONDS", "25"))


def b64(data: bytes) -> str:
    # Padding is kept: storage/utils.py does not strip it, and the MAC has to
    # match what the server recomputes from the header verbatim.
    return b64encode(data).decode("ascii")


class Client:
    """Minimal direct-access Hawk client, mirroring storage/client.py."""

    def __init__(self, server_url: str):
        url = urlparse(server_url)
        if not url.fragment:
            raise SystemExit("SERVER_URL must carry a #<master-secret> fragment")
        self.uid = random.randint(1, 1000000)
        secret = url.fragment
        endpoint = url._replace(
            path=url.path.rstrip("/") + "/1.5/" + str(self.uid), fragment=""
        )
        self.endpoint_url = endpoint.geturl()
        data = {
            "uid": self.uid,
            "fxa_uid": hashlib.sha256(
                f"{self.uid}:fxa_uid".encode("ascii")
            ).hexdigest(),
            "fxa_kid": hashlib.sha256(
                f"{self.uid}:fxa_kid".encode("ascii")
            ).hexdigest()[:32],
            "hashed_fxa_uid": hashlib.sha256(
                f"{self.uid}:hashed_fxa_uid".encode("ascii")
            ).hexdigest(),
            "node": url._replace(path="", fragment="").geturl(),
            "expires": time.time() + 3600,
        }
        token = make_token(data, secret=secret)
        self.auth_token = token
        self.auth_secret = derive(token, secret=secret).encode("ascii")
        parsed = urlparse(self.endpoint_url)
        self.host = parsed.hostname
        self.port = str(parsed.port or (80 if parsed.scheme == "http" else 443))
        self.host_header = parsed.netloc
        self.timeskew = 0

    def _auth_header(self, method: str, url: str) -> str:
        parsed = urlparse(url)
        path_qs = parsed.path + ("?" + parsed.query if parsed.query else "")
        params = {
            "id": self.auth_token,
            "ts": str(int(time.time()) + self.timeskew),
            "nonce": b64(os.urandom(5)),
        }
        sigstr = "\n".join(
            [
                "hawk.1.header",
                params["ts"],
                params["nonce"],
                method,
                path_qs,
                self.host.lower(),
                self.port,
                "",
                "",
                "",
            ]
        )
        params["mac"] = b64(
            hmac.new(self.auth_secret, sigstr.encode("ascii"), hashlib.sha256).digest()
        )
        return "Hawk " + ", ".join(f'{k}="{v}"' for k, v in params.items())

    def request(self, method: str, path: str, body=None):
        url = self.endpoint_url + path

        def send():
            headers = {
                "Authorization": self._auth_header(method, url),
                "Host": self.host_header,
                "Content-Type": "application/json",
            }
            return requests.request(
                method, url, headers=headers, data=body, timeout=120
            )

        resp = send()
        # Same correction the real client makes: a 401 carries the server's
        # clock in X-Weave-Timestamp, so re-sign once against it.
        if resp.status_code == 401 and "X-Weave-Timestamp" in resp.headers:
            server_time = int(float(resp.headers["X-Weave-Timestamp"]))
            self.timeskew = server_time - int(time.time())
            resp = send()
        return resp


GCS_HOST = os.environ.get("PROBE_GCS_HOST", "http://fake-gcs:4443")
GCS_BUCKET = os.environ.get("PROBE_GCS_BUCKET", "test-payloads")


def gcs_objects(fxa_uid: str) -> list:
    """List this user's payload objects, so the probe can say at which stage an
    object appears and disappears rather than only that it is gone at the end.
    """
    try:
        resp = requests.get(
            f"{GCS_HOST}/storage/v1/b/{GCS_BUCKET}/o",
            params={"prefix": f"{fxa_uid}/", "maxResults": "100"},
            timeout=30,
        )
        return resp.json().get("items", []) or []
    except Exception as e:  # noqa: BLE001
        print(f"probe:   (gcs list failed: {e})")
        return []


def show_gcs(stage: str, fxa_uid: str) -> None:
    items = gcs_objects(fxa_uid)
    print(f"probe: [{stage}] gcs objects: {len(items)}")
    for it in items:
        committed = (it.get("metadata") or {}).get("committed", "<unset>")
        print(
            f"probe:   {it.get('name')} size={it.get('size')} "
            f"committed={committed} customTime={it.get('customTime', '<unset>')}"
        )


def main() -> int:
    c = Client(SERVER_URL)
    payload = "p" * PAYLOAD_SIZE
    bso_id = "probe-batch-commit"
    fxa_uid = hashlib.sha256(f"{c.uid}:fxa_uid".encode("ascii")).hexdigest()

    print(f"probe: uid={c.uid} fxa_uid={fxa_uid[:16]}...")
    print(f"probe: collection={COLLECTION} payload={PAYLOAD_SIZE}B")

    # Two-request transactional batch, so the commit is a real handoff from
    # batch_bsos to bsos rather than the single-request optimisation.
    resp = c.request(
        "POST",
        f"/storage/{COLLECTION}?batch=true",
        json.dumps([{"id": bso_id, "payload": payload}]),
    )
    print(f"probe: batch create -> {resp.status_code}")
    if resp.status_code != 202:
        print(f"probe: unexpected batch-create response: {resp.text[:400]}")
        return 1
    batch_id = resp.json()["batch"]
    show_gcs("after batch create", fxa_uid)

    resp = c.request(
        "POST",
        f"/storage/{COLLECTION}?commit=true&batch={batch_id}",
        json.dumps([{"id": bso_id + "-2", "payload": payload}]),
    )
    print(f"probe: batch commit -> {resp.status_code}")
    if resp.status_code != 200:
        print(f"probe: unexpected commit response: {resp.text[:400]}")
        return 1
    show_gcs("immediately after commit", fxa_uid)

    print(f"probe: waiting {SETTLE_SECONDS}s for the reconciler to act...")
    time.sleep(SETTLE_SECONDS)
    show_gcs("after settle", fxa_uid)

    failed = []
    for bid in (bso_id, bso_id + "-2"):
        resp = c.request("GET", f"/storage/{COLLECTION}/{bid}")
        ok = resp.status_code == 200
        size = len(resp.json().get("payload", "")) if ok else 0
        print(f"probe: GET {bid} -> {resp.status_code} payload={size}B")
        if not ok or size != PAYLOAD_SIZE:
            failed.append(bid)
            if not ok:
                print(f"probe:   body: {resp.text[:300]}")

    if failed:
        print(
            "probe: FAIL -- committed batch payload(s) unreadable after "
            f"reconciliation: {failed}"
        )
        print(
            "probe: the reconciler deleted a GCS object that a committed bsos "
            "row still points at (STOR-657 shape)."
        )
        return 1
    print("probe: PASS -- committed batch payloads survived reconciliation.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
