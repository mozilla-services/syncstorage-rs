#!/usr/bin/env python3
"""Mint an assertion for the syncstorage data deletion endpoint.

In production the FxA Auth Server mints these. This exists so operators can
exercise `DELETE /1.0/users/{fxa_uid}/storage` by hand, and so the integration
tests have something to sign with; it is also the reference for the claim shape
tokenserver expects.

The assertion is an RS256 JWT verified against the same FxA JWKs tokenserver
already uses for OAuth. It names its target in `sub` -- tokenserver acts on
that, not on the uid in the path, and rejects the request if the two disagree.
"""

import argparse
import os
import time
import uuid

import jwt

#: Must match DELETE_ASSERTION_TYP in tokenserver-auth's crypto module, modulo
#: the implicit `application/` prefix the server applies (RFC 7515 4.1.9).
#: Signed unprefixed, the way FxA emits it.
TYP = "fxa-sync-delete+jwt"

DEFAULT_AUDIENCE = "urn:mozilla:fxa:tokenserver:sync-data-delete"
DEFAULT_ISSUER = "https://accounts.firefox.com"

#: The server caps how far ahead `exp` may sit, so this has to stay well under
#: DELETE_ASSERTION_MAX_LIFETIME_SECS.
DEFAULT_TTL_SECONDS = 30


def make_delete_assertion(
    private_key_pem: bytes,
    fxa_uid: str,
    audience: str = DEFAULT_AUDIENCE,
    issuer: str = DEFAULT_ISSUER,
    ttl: int = DEFAULT_TTL_SECONDS,
    typ: str = TYP,
    jti: str | None = None,
) -> str:
    """Return the `Authorization: Bearer` value authorizing deletion for `fxa_uid`."""
    now = int(time.time())
    claims = {
        "sub": fxa_uid,
        "aud": audience,
        "iss": issuer,
        "iat": now,
        "exp": now + ttl,
    }
    # Required by the server and logged for correlation, but deliberately not
    # deduplicated; see ADR-0052. `exp` is what bounds replay.
    if jti is not None:
        claims["jti"] = jti
    # PyJWT defaults the header to `typ: JWT`, which the server rejects.
    return jwt.encode(claims, private_key_pem, algorithm="RS256", headers={"typ": typ})


def main() -> None:
    """Print an assertion, or a ready-to-run curl command with --curl."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fxa_uid", help="the FxA account id whose data to delete")
    parser.add_argument(
        "--key",
        default=os.environ.get("FXA_DELETE_SIGNING_KEY_PEM"),
        help="RSA private key PEM; defaults to $FXA_DELETE_SIGNING_KEY_PEM",
    )
    parser.add_argument("--audience", default=DEFAULT_AUDIENCE)
    parser.add_argument("--issuer", default=DEFAULT_ISSUER)
    parser.add_argument("--ttl", type=int, default=DEFAULT_TTL_SECONDS)
    parser.add_argument(
        "--host",
        default=os.environ.get("TOKENSERVER_HOST", "http://localhost:8000"),
        help="tokenserver base url, used by --curl",
    )
    parser.add_argument(
        "--curl", action="store_true", help="print a full curl command instead"
    )
    args = parser.parse_args()

    if not args.key:
        parser.error("no signing key; pass --key or set FXA_DELETE_SIGNING_KEY_PEM")

    try:
        with open(args.key, "rb") as handle:
            pem = handle.read()
    except OSError as exc:
        parser.error(f"could not read the signing key: {exc}")

    token = make_delete_assertion(
        pem,
        args.fxa_uid,
        audience=args.audience,
        issuer=args.issuer,
        ttl=args.ttl,
        jti=str(uuid.uuid4()),
    )
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
