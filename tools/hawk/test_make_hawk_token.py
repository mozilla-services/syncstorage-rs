import argparse
import time
from typing import Any

import hawkauthlib
import pytest
import tokenlib
from webob.request import Request

from make_hawk_token import create_token, metrics_hash

DEFAULT_ARGS = argparse.Namespace(
    uid=1,
    node="http://localhost:8000",
    uri="/1.5/1/storage/col2/",
    method="GET",
    duration=3600,
    fxa_uid="DEADBEEF00004be4ae957006c0ceb620",
    fxa_kid="DEADBEEF00004be4ae957006c0ceb620",
    device_id="device1",
    secret="Ted_Koppel_is_a_robot",
    hmac_key=b"foo",
)


def make_args(**overrides: Any) -> argparse.Namespace:
    """Return a Namespace based on DEFAULT_ARGS with any fields overridden."""
    return argparse.Namespace(**{**vars(DEFAULT_ARGS), **overrides})


# --- metrics_hash ---


def test_metrics_hash_returns_hex_string() -> None:
    """metrics_hash output must be a valid hexadecimal string."""
    result = metrics_hash(make_args(), "somevalue")
    assert isinstance(result, str)
    int(result, 16)  # raises ValueError if not valid hex


def test_metrics_hash_is_deterministic() -> None:
    """Same input always produces the same hash."""
    assert metrics_hash(make_args(), "abc") == metrics_hash(make_args(), "abc")


def test_metrics_hash_strips_email_domain() -> None:
    """Only the local part before '@' is hashed; the domain is ignored."""
    local = metrics_hash(make_args(), "user")
    email = metrics_hash(make_args(), "user@example.com")
    assert local == email


def test_metrics_hash_str_hmac_key_coerced_to_bytes() -> None:
    """A string hmac_key is coerced to bytes and produces the correct HMAC-SHA256 digest."""
    # Expected: HMAC-SHA256(key=b"foo", msg=b"value") via hmac.new(b'foo', b'value', hashlib.sha256).hexdigest())
    expected = "40036668289b3e3257fd7653c09d3c0611d5f6e813b674c2b3984005e5736019"
    args = make_args(hmac_key="foo")
    result = metrics_hash(args, "value")
    assert result == expected
    assert args.hmac_key == b"foo"


# --- create_token ---


def test_create_token_returns_four_tuple() -> None:
    """create_token returns (token, key, expires, salt)."""
    assert len(create_token(make_args())) == 4


def test_create_token_expires_in_future() -> None:
    """The expiry timestamp embedded in the token is after the current time."""
    args = make_args()
    token, _, _, _ = create_token(args)
    decoded = tokenlib.parse_token(token, secret=args.secret)
    assert decoded["expires"] > int(time.time())


def test_create_token_expires_approximately_correct() -> None:
    """The expiry is within 5 seconds of now + duration."""
    now = int(time.time())
    _, _, expires, _ = create_token(make_args(duration=3600))
    assert abs(expires - (now + 3600)) < 5


def test_create_token_salt_is_hex_string() -> None:
    """The salt returned by create_token is a valid hexadecimal string."""
    _, _, _, salt = create_token(make_args())
    assert isinstance(salt, str)
    int(salt, 16)


def test_create_token_data_roundtrips() -> None:
    """Token payload decoded with tokenlib contains the original uid, node, and FxA fields."""
    args = make_args()
    token, _, _, _ = create_token(args)
    decoded = tokenlib.parse_token(token, secret=args.secret)
    assert decoded["uid"] == args.uid
    assert decoded["node"] == args.node
    assert decoded["fxa_uid"] == args.fxa_uid
    assert decoded["fxa_kid"] == args.fxa_kid


def test_create_token_derived_key_matches() -> None:
    """The key returned by create_token equals the tokenlib-derived secret for that token."""
    args = make_args()
    token, key, _, _ = create_token(args)
    assert key == tokenlib.get_derived_secret(token, secret=args.secret)


# --- hawk header signing ---


def test_hawk_header_is_produced() -> None:
    """sign_request returns a non-empty Hawk authorization header."""
    args = make_args()
    token, key, _, _ = create_token(args)
    req = Request.blank(args.node + args.uri)
    req.method = args.method
    header = hawkauthlib.sign_request(req, token, key)
    assert header.startswith("Hawk ")


def test_hawk_header_contains_required_fields() -> None:
    """Hawk header includes the id, mac, and ts fields required by the protocol."""
    args = make_args()
    token, key, _, _ = create_token(args)
    req = Request.blank(args.node + args.uri)
    req.method = args.method
    header = hawkauthlib.sign_request(req, token, key)
    assert "id=" in header
    assert "mac=" in header
    assert "ts=" in header


def test_hawk_mac_differs_by_method() -> None:
    """PUT and GET produce different MACs for the same token and URI."""
    token, key, _, _ = create_token(make_args())
    path = DEFAULT_ARGS.node + DEFAULT_ARGS.uri

    req_get = Request.blank(path)
    req_get.method = "GET"
    req_put = Request.blank(path)
    req_put.method = "PUT"

    assert hawkauthlib.sign_request(req_get, token, key) != hawkauthlib.sign_request(
        req_put, token, key
    )


def test_hawk_mac_differs_by_uri() -> None:
    """Different URI paths produce different MACs for the same token."""
    token, key, _, _ = create_token(make_args())

    req_a = Request.blank(DEFAULT_ARGS.node + "/1.5/1/storage/col2/")
    req_b = Request.blank(DEFAULT_ARGS.node + "/1.5/1/storage/meta/global")
    req_a.method = req_b.method = "GET"

    assert hawkauthlib.sign_request(req_a, token, key) != hawkauthlib.sign_request(
        req_b, token, key
    )


def test_different_secrets_produce_distinct_tokens() -> None:
    """Tokens created with different secrets cannot be decoded by the other secret."""
    token_a, _, _, _ = create_token(make_args(secret="secret-a"))
    token_b, _, _, _ = create_token(make_args(secret="secret-b"))

    # Each token is valid under its own secret
    tokenlib.parse_token(token_a, secret="secret-a")
    tokenlib.parse_token(token_b, secret="secret-b")

    # Cross-validation must fail
    with pytest.raises(tokenlib.errors.InvalidSignatureError):
        tokenlib.parse_token(token_a, secret="secret-b")


# --- main() output modes ---


def test_main_default_output(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """Default output (no --as_header) prints Expires, Salt, Path, and Hawk header label."""
    import make_hawk_token

    monkeypatch.setattr(
        "sys.argv",
        ["make_hawk_token.py", "--secret", "Ted_Koppel_is_a_robot"],
    )
    make_hawk_token.main()
    out = capsys.readouterr().out
    assert "Expires:" in out
    assert "Salt:" in out
    assert "Path:" in out
    assert "Hawk Authorization Header:" in out


def test_main_as_header_output(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    """--as_header prints exactly one line: 'Authorization: Hawk ...'."""
    import make_hawk_token

    monkeypatch.setattr(
        "sys.argv",
        ["make_hawk_token.py", "--secret", "Ted_Koppel_is_a_robot", "--as_header"],
    )
    make_hawk_token.main()
    lines = [line for line in capsys.readouterr().out.splitlines() if line.strip()]
    assert len(lines) == 1
    assert lines[0].startswith("Authorization: Hawk ")
