#!/usr/bin/env python3
r"""Mint a credential for the syncstorage data deletion endpoint.

This is the reference implementation of the canonical string that
``DELETE /1.0/users/{fxa_uid}/storage`` authenticates against, for operators
testing by hand and for the FxA Auth Server team to port.

The credential is *not* the shared secret. It is an HMAC over the uid being
deleted and an expiry, so a captured header only authorizes deleting that one
uid, and only until it expires::

    Authorization: Bearer <exp>.<base64url_nopad(hmac)>
    hmac = HMAC-SHA256(secret, "syncstorage-delete/v1\n<fxa_uid>\n<exp>")

Mint one per HTTP attempt rather than per logical operation: a credential
reused across a retry with backoff will expire mid-sequence and 401.
"""

import argparse
import base64
import hashlib
import hmac
import os
import time

#: Domain separation, keeping this MAC from being replayable against any other
#: use of the same key. Must match DELETE_TOKEN_DOMAIN in
#: syncserver/src/tokenserver/extractors.rs.
DOMAIN = "syncstorage-delete/v1"

#: The server rejects an exp more than 120s out, so stay well inside that.
DEFAULT_TTL_SECONDS = 60


def make_delete_token(secret: str, fxa_uid: str, ttl: int = DEFAULT_TTL_SECONDS) -> str:
    """Return the ``Authorization: Bearer`` value for deleting ``fxa_uid``."""
    exp = int(time.time()) + ttl
    # Newline delimited so a uid ending in digits cannot be confused with a
    # shifted exp.
    message = f"{DOMAIN}\n{fxa_uid}\n{exp}".encode()
    mac = hmac.new(secret.encode(), message, hashlib.sha256).digest()
    return f"{exp}.{base64.urlsafe_b64encode(mac).rstrip(b'=').decode()}"


def main() -> None:
    """Print a credential, or a ready-to-run curl command with --curl."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fxa_uid", help="the FxA account id whose data to delete")
    parser.add_argument(
        "--secret",
        default=os.environ.get("SYNC_TOKENSERVER__FXA_DELETE_DATA_SECRETS"),
        help="shared secret; defaults to $SYNC_TOKENSERVER__FXA_DELETE_DATA_SECRETS",
    )
    parser.add_argument(
        "--ttl", type=int, default=DEFAULT_TTL_SECONDS, help="seconds until expiry"
    )
    parser.add_argument(
        "--host",
        default=os.environ.get("TOKENSERVER_HOST", "http://localhost:8000"),
        help="tokenserver base url, used by --curl",
    )
    parser.add_argument(
        "--curl", action="store_true", help="print a full curl command instead"
    )
    args = parser.parse_args()

    if not args.secret:
        parser.error(
            "no secret given; pass --secret or set "
            "SYNC_TOKENSERVER__FXA_DELETE_DATA_SECRETS"
        )

    token = make_delete_token(args.secret, args.fxa_uid, args.ttl)
    if args.curl:
        print(
            f"curl -i -X DELETE "
            f'-H "Authorization: Bearer {token}" '
            f'"{args.host}/1.0/users/{args.fxa_uid}/storage"'
        )
    else:
        print(token)


if __name__ == "__main__":
    main()
