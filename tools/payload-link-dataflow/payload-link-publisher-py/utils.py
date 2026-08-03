# Utility helpers for the payload-link publisher.
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
"""Small helpers shared by the publisher script and its tests."""

import json
from typing import Any


def extract_payload_link(values_json: Any) -> str | None:
    """Extract ``payload_link`` from a Spanner change-stream values blob.

    Accepts either a JSON-encoded string (prod / Java wire shape) or an
    already-parsed dict-like object (which is what the Python Spanner
    client returns for ``JSON`` columns via the emulator's
    ``JsonObject`` wrapper). Returns ``None`` when the value is
    null/empty, the ``payload_link`` key is absent, or the value is
    JSON-null / empty string. Returns the literal sentinel
    ``"<malformed>"`` for un-parseable strings so the caller can treat
    malformed records as actionable (they pass through to the
    reconciler / DLQ).
    """
    if values_json is None or values_json == "":
        return None
    if isinstance(values_json, str):
        try:
            parsed: Any = json.loads(values_json)
        except (json.JSONDecodeError, TypeError):
            return "<malformed>"
    else:
        # Already parsed by the client (e.g. a JsonObject / dict).
        parsed = values_json
    if not isinstance(parsed, dict):
        return None
    value = parsed.get("payload_link")
    if value is None or value == "":
        return None
    return str(value)


def is_payload_link_actionable(mods: list[dict[str, Any]]) -> bool:
    """Determine iff at least one mod references a non-null ``payload_link`` on either side.

    Mirrors ``PayloadLinkChangesToPubSub.isPayloadLinkActionable`` in the
    Java flex template. Records whose every mod has both old and new
    ``payload_link`` NULL are inert for the reconciler and get dropped.
    Malformed JSON in either side passes through so the reconciler /
    DLQ surfaces it.
    """
    for mod in mods:
        old = extract_payload_link(mod.get("old_values"))
        new = extract_payload_link(mod.get("new_values"))
        if old is not None or new is not None:
            return True
    return False
