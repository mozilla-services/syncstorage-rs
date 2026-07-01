# Utility helpers for the payload reconciler.
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
"""Small utilities shared by the reconciler script and its tests."""


def parse_gs_url(url: str) -> tuple[str, str]:
    """Split a ``gs://bucket/object`` URL into ``(bucket, object)``.

    Mirrors the canonical implementation in
    ``syncserver/src/web/payload_offload.rs::parse_gs_url`` so producer
    and reconciler agree on the URL grammar.

    Raises:
        ValueError: if the URL is not of the form ``gs://<bucket>/<object>``
            with both parts non-empty.
    """
    if not url.startswith("gs://"):
        raise ValueError(f"not a gs:// URL: {url!r}")
    body = url[len("gs://") :]
    bucket, sep, name = body.partition("/")
    if not sep or not bucket or not name:
        raise ValueError(f"malformed gs:// URL: {url!r}")
    return bucket, name
