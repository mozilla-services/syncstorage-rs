"""Tests for utils.py."""

import pytest

from utils import parse_gs_url


def test_parse_gs_url_simple() -> None:
    assert parse_gs_url("gs://my-bucket/object-name") == ("my-bucket", "object-name")


def test_parse_gs_url_nested_object() -> None:
    assert parse_gs_url("gs://my-bucket/fxa/history/bso/uuid-1") == (
        "my-bucket",
        "fxa/history/bso/uuid-1",
    )


def test_parse_gs_url_rejects_non_gs_scheme() -> None:
    with pytest.raises(ValueError, match="not a gs:// URL"):
        parse_gs_url("s3://my-bucket/object")


def test_parse_gs_url_rejects_empty_object() -> None:
    with pytest.raises(ValueError, match="malformed gs:// URL"):
        parse_gs_url("gs://my-bucket/")


def test_parse_gs_url_rejects_empty_bucket() -> None:
    with pytest.raises(ValueError, match="malformed gs:// URL"):
        parse_gs_url("gs:///object")


def test_parse_gs_url_rejects_bucket_only() -> None:
    with pytest.raises(ValueError, match="malformed gs:// URL"):
        parse_gs_url("gs://my-bucket")
