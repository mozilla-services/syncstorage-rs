#!/usr/bin/env python3
"""Create a Hawk token for tests

requires hawkauthlib, tokenlib, webob

Creates the hawk headers for auth::tests, in particular valid_header and
valid_header_with_querystring.

The latter modifies the query string which changes the mac/nonce and
potentially its values (in the Hawk header).
"""

import argparse
import hmac
import os
import time
from binascii import hexlify
from datetime import timedelta
from hashlib import sha256

import hawkauthlib
import tokenlib
from webob.request import Request

LEGACY_UID = 1
COL = "col2"
URI = f"/1.5/{LEGACY_UID}/storage/{COL}/"
METHOD = "GET"
FXA_UID = "DEADBEEF00004be4ae957006c0ceb620"
FXA_KID = "DEADBEEF00004be4ae957006c0ceb620"
DEVICE_ID = "device1"
NODE = "http://localhost:8000"
SECRET = os.environ.get("SYNC_MASTER_SECRET", "Ted_Koppel_is_a_robot")
HMAC_KEY = b"foo"

# 10 years
DURATION = timedelta(days=10 * 365).total_seconds()

SALT = hexlify(os.urandom(3)).decode("ascii")


def get_args() -> argparse.Namespace:
    """Parse and return command-line arguments."""
    parser = argparse.ArgumentParser(
        description="Create a hawk header for use in testing"
    )
    parser.add_argument(
        "--uid", type=int, default=LEGACY_UID, help=f"Legacy UID ({LEGACY_UID})"
    )
    parser.add_argument("--uri", default=URI, help=f"URI path ({URI})")
    parser.add_argument(
        "--method", default=METHOD, help=f"The HTTP Method ({METHOD})"
    )
    parser.add_argument(
        "--fxa_uid", default=FXA_UID, help=f"FxA User ID ({FXA_UID})"
    )
    parser.add_argument(
        "--fxa_kid", default=FXA_KID, help=f"FxA K ID ({FXA_KID})"
    )
    parser.add_argument(
        "--device_id", default=DEVICE_ID, help=f"FxA Device ID ({DEVICE_ID})"
    )
    parser.add_argument(
        "--node", default=NODE, help=f"HTTP Host URI for node ({NODE})"
    )
    parser.add_argument(
        "--duration", type=int, default=DURATION, help=f"Hawk TTL ({DURATION})"
    )
    parser.add_argument(
        "--secret", default=SECRET, help=f"Shared HAWK secret ({SECRET})"
    )
    parser.add_argument(
        "--hmac_key",
        default=HMAC_KEY,
        help="HAWK HMAC key ({!r})".format(HMAC_KEY),
    )
    parser.add_argument(
        "--as_header",
        action="store_true",
        default=False,
        help="return only header (False)",
    )
    return parser.parse_args()


def create_token(args: argparse.Namespace) -> tuple[str, str, int, str]:
    """Build a tokenlib token and derive its signing key.

    Returns (token, key, expires, salt).
    """
    expires = int(time.time()) + args.duration
    token_data = {
        "uid": args.uid,
        "node": args.node,
        "expires": expires,
        "fxa_uid": args.fxa_uid,
        "fxa_kid": args.fxa_kid,
        "hashed_fxa_uid": metrics_hash(args, args.fxa_uid),
        "hashed_device_id": metrics_hash(args, args.device_id),
        "salt": SALT,
    }
    token: str = tokenlib.make_token(token_data, secret=args.secret)
    key: str = tokenlib.get_derived_secret(token, secret=args.secret)
    return token, key, expires, SALT


def metrics_hash(args: argparse.Namespace, value: str) -> str:
    """Return the HMAC-SHA256 hex digest of value, using only the local part of email addresses."""
    if isinstance(args.hmac_key, str):
        args.hmac_key = args.hmac_key.encode()
    hasher = hmac.new(args.hmac_key, b"", sha256)
    # value may be an email address, in which case we only want the first part
    hasher.update(value.encode("utf-8").split(b"@", 1)[0])
    return hasher.hexdigest()


def main() -> None:
    """Entry point: parse args, create a token, and print the Hawk authorization header."""
    args = get_args()
    token, key, expires, salt = create_token(args)
    path = "{node}{uri}".format(node=args.node, uri=args.uri)
    req = Request.blank(path)
    req.method = args.method
    header = hawkauthlib.sign_request(req, token, key)
    if not args.as_header:
        print("Expires: ", expires)
        print("Salt: ", salt)
        print("\nPath: ", path)
        print("Hawk Authorization Header: ", header)
    else:
        print("Authorization:", header)


if __name__ == "__main__":
    main()
