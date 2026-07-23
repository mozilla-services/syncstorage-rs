"""Unit tests for the Python payload-link publisher.

Covers the pure functions: ``extract_payload_link``,
``is_payload_link_actionable``, ``serialize_record``. Does not exercise
``poll_change_stream`` (integration territory; needs a live Spanner
emulator) or ``publish_if_actionable`` (needs a Pub/Sub emulator).
"""

import datetime
import json

from utils import extract_payload_link, is_payload_link_actionable
from publisher import serialize_record


# ---------------- extract_payload_link ----------------


def test_extract_payload_link_returns_value() -> None:
    assert (
        extract_payload_link('{"payload_link":"gs://b/u/c/bso/uuid"}')
        == "gs://b/u/c/bso/uuid"
    )


def test_extract_payload_link_returns_none_for_empty_input() -> None:
    assert extract_payload_link("") is None
    assert extract_payload_link(None) is None


def test_extract_payload_link_returns_none_when_key_absent() -> None:
    assert extract_payload_link("{}") is None
    assert extract_payload_link('{"payload":"inline"}') is None


def test_extract_payload_link_returns_none_when_value_null() -> None:
    assert extract_payload_link('{"payload_link":null}') is None


def test_extract_payload_link_returns_none_when_value_empty_string() -> None:
    assert extract_payload_link('{"payload_link":""}') is None


def test_extract_payload_link_returns_sentinel_when_malformed() -> None:
    # Malformed JSON must NOT be treated as inert -- pass through.
    assert extract_payload_link('{"not-json') == "<malformed>"


def test_extract_payload_link_accepts_already_parsed_dict() -> None:
    # Emulator returns JSON columns as parsed dicts (JsonObject), not
    # JSON-encoded strings.
    assert extract_payload_link({"payload_link": "gs://b/u/c/bso/uuid"}) == (
        "gs://b/u/c/bso/uuid"
    )
    assert extract_payload_link({"payload_link": None}) is None
    assert extract_payload_link({}) is None
    assert extract_payload_link({"other_key": "value"}) is None


# ---------------- is_payload_link_actionable ----------------


LINK_A = '{"payload_link":"gs://b/u/c/bso/uuid-a"}'
LINK_B = '{"payload_link":"gs://b/u/c/bso/uuid-b"}'
NULL_LINK = '{"payload_link":null}'
NO_LINK = "{}"


def _mod(old: str | None, new: str | None) -> dict[str, str | None]:
    return {"keys": "{}", "old_values": old, "new_values": new}


def test_actionable_insert_with_link() -> None:
    assert is_payload_link_actionable([_mod(None, LINK_A)]) is True


def test_actionable_delete_with_old_link() -> None:
    assert is_payload_link_actionable([_mod(LINK_A, None)]) is True


def test_actionable_update_replace() -> None:
    assert is_payload_link_actionable([_mod(LINK_A, LINK_B)]) is True


def test_inert_both_null() -> None:
    assert is_payload_link_actionable([_mod(NULL_LINK, NULL_LINK)]) is False


def test_inert_empty_maps() -> None:
    assert is_payload_link_actionable([_mod(NO_LINK, NO_LINK)]) is False


def test_multi_mod_any_actionable_wins() -> None:
    assert (
        is_payload_link_actionable(
            [_mod(NULL_LINK, NULL_LINK), _mod(None, LINK_A), _mod(NO_LINK, NO_LINK)]
        )
        is True
    )


def test_multi_mod_all_inert_drops() -> None:
    assert (
        is_payload_link_actionable(
            [
                _mod(NULL_LINK, NULL_LINK),
                _mod(NO_LINK, NO_LINK),
                _mod(NULL_LINK, NO_LINK),
            ]
        )
        is False
    )


def test_malformed_json_passes_through() -> None:
    # A malformed values blob must not silently drop the record.
    assert is_payload_link_actionable([_mod("{not-json", None)]) is True


# ---------------- serialize_record ----------------


def test_serialize_record_wire_shape() -> None:
    dcr = {
        "commit_timestamp": datetime.datetime(2026, 6, 30, tzinfo=datetime.UTC),
        "mod_type": "UPDATE",
        "table_name": "bsos",
        "mods": [
            {"keys": '{"bso_id":"x"}', "old_values": LINK_A, "new_values": LINK_B}
        ],
    }

    out = json.loads(serialize_record(dcr))

    assert out["commitTimestamp"] == "2026-06-30T00:00:00+00:00"
    assert out["modType"] == "UPDATE"
    assert out["tableName"] == "bsos"
    assert len(out["mods"]) == 1
    m = out["mods"][0]
    # Mod fields are JSON *strings*, not nested objects. Consumer double-parses.
    assert isinstance(m["keys"], str)
    assert isinstance(m["oldValues"], str)
    assert isinstance(m["newValues"], str)
    assert json.loads(m["oldValues"]) == {"payload_link": "gs://b/u/c/bso/uuid-a"}
    assert json.loads(m["newValues"]) == {"payload_link": "gs://b/u/c/bso/uuid-b"}


def test_serialize_record_defaults_empty_values_to_empty_object_string() -> None:
    """None old/new_values must become the string ``"{}"`` (matches Java)."""
    dcr = {
        "commit_timestamp": datetime.datetime(2026, 6, 30, tzinfo=datetime.UTC),
        "mod_type": "DELETE",
        "table_name": "bsos",
        "mods": [{"keys": "{}", "old_values": LINK_A, "new_values": None}],
    }
    out = json.loads(serialize_record(dcr))
    assert out["mods"][0]["newValues"] == "{}"


def test_serialize_record_handles_missing_mods_key() -> None:
    dcr = {
        "commit_timestamp": datetime.datetime(2026, 6, 30, tzinfo=datetime.UTC),
        "mod_type": "UPDATE",
        "table_name": "bsos",
    }
    out = json.loads(serialize_record(dcr))
    assert out["mods"] == []


def test_serialize_record_encodes_parsed_dict_mod_values() -> None:
    """Mod values that come back parsed (JsonObject) get re-encoded to strings."""
    dcr = {
        "commit_timestamp": datetime.datetime(2026, 6, 30, tzinfo=datetime.UTC),
        "mod_type": "UPDATE",
        "table_name": "bsos",
        "mods": [
            {
                "keys": {"bso_id": "x"},
                "old_values": {"payload_link": "gs://b/u/c/bso/uuid-a"},
                "new_values": {"payload_link": "gs://b/u/c/bso/uuid-b"},
            }
        ],
    }
    out = json.loads(serialize_record(dcr))
    m = out["mods"][0]
    # Wire format still carries JSON strings, regardless of input form.
    assert isinstance(m["keys"], str)
    assert json.loads(m["keys"]) == {"bso_id": "x"}
    assert json.loads(m["oldValues"]) == {"payload_link": "gs://b/u/c/bso/uuid-a"}
    assert json.loads(m["newValues"]) == {"payload_link": "gs://b/u/c/bso/uuid-b"}


def test_serialize_record_string_timestamp_pass_through() -> None:
    dcr = {
        "commit_timestamp": "2026-06-30T00:00:00Z",
        "mod_type": "INSERT",
        "table_name": "batch_bsos",
        "mods": [],
    }
    out = json.loads(serialize_record(dcr))
    assert out["commitTimestamp"] == "2026-06-30T00:00:00Z"
