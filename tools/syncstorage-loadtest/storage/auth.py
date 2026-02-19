"""FxA authentication and JWT token management."""

import hashlib
import json
import os
import random
import string
import time
from pathlib import Path
from typing import Any, Optional

import jwt
from cryptography.hazmat.backends import default_backend
from cryptography.hazmat.primitives import serialization
from fxa.core import Client
from fxa.oauth import Client as OAuthClient
from fxa.tests.utils import TestEmailAccount

# Client ID for Firefox Desktop
CLIENT_ID = "5882386c6d801776"
FXA_API_HOST = os.environ.get("FXA_API_HOST", "https://api-accounts.stage.mozaws.net")
FXA_OAUTH_HOST = os.environ.get("FXA_OAUTH_HOST", "https://oauth.stage.mozaws.net")
OAUTH_SCOPE = "https://identity.mozilla.com/apps/oldsync"
PASSWORD_LENGTH = 20

# Assertions are good for one year (in seconds).
# This avoids having to deal with clock-skew in tokenserver requests.
ASSERTION_LIFETIME = 60 * 60 * 24 * 365

# OAuth JWT signing to test without FxA
OAUTH_PRIVATE_KEY_FILE = os.environ.get("OAUTH_PRIVATE_KEY_FILE")
OAUTH_ISSUER = os.environ.get("OAUTH_ISSUER", "http://mock-fxa-server:6000")
OAUTH_JWT_ALGORITHM = os.environ.get("OAUTH_JWT_ALGORITHM", "RS256")

_ACCT_TRACKING_FILE = Path(__file__).parent.parent / ".accounts_tracking.json"


def _generate_password() -> str:
    """Generate a random password for FxA account.

    Returns:
        str: Random password string.

    """
    return "".join(random.choice(string.printable) for i in range(PASSWORD_LENGTH))


def _track_account_creation(email: str, password: str, fxa_uid: str) -> None:
    """Track FxA account creation in a local file for cleanup.

    Args:
        email: Account email address.
        password: Account password.
        fxa_uid: FxA user ID.

    """
    try:
        if _ACCT_TRACKING_FILE.exists():
            with open(_ACCT_TRACKING_FILE, "r") as f:
                accounts = json.load(f)
        else:
            accounts = []

        for acc in accounts:
            # already tracked
            if acc["email"] == email:
                return

        accounts.append(
            {
                "email": email,
                "password": password,
                "fxa_uid": fxa_uid,
                "created_at": int(time.time()),
            }
        )

        with open(_ACCT_TRACKING_FILE, "w") as f:
            json.dump(accounts, f, indent=2)

    except Exception:
        # continue with tests
        pass


def _remove_account_from_tracking(email: str) -> None:
    """Remove an account from the tracking file.

    Args:
        email: Account email address to remove.

    """
    try:
        if not _ACCT_TRACKING_FILE.exists():
            return

        with open(_ACCT_TRACKING_FILE, "r") as f:
            accounts = json.load(f)

        accounts = [acc for acc in accounts if acc["email"] != email]

        if not accounts:
            _ACCT_TRACKING_FILE.unlink()
        else:
            with open(_ACCT_TRACKING_FILE, "w") as f:
                json.dump(accounts, f, indent=2)

    except Exception:
        pass


def _create_self_signed_jwt(
    email: str, client_id: str = CLIENT_ID
) -> tuple[str, str, str, str, None, None, None]:
    """Create a self-signed OAuth JWT.

    Requires OAUTH_PRIVATE_KEY_FILE env var.

    Args:
        email: Email address for the JWT.
        client_id: OAuth client ID (default: CLIENT_ID constant).

    Returns:
        tuple: A tuple containing:
            - oauth_token: Self-signed JWT
            - email: Email address
            - fxa_uid: Generated FxA uid
            - key_id: Key id
            - None: FxA session not needed
            - None: OAuth client not needed
            - None: cleanup info not needed

    """
    if not OAUTH_PRIVATE_KEY_FILE:
        raise ValueError("OAUTH_PRIVATE_KEY_FILE must be set")

    with open(OAUTH_PRIVATE_KEY_FILE, "rb") as f:
        private_key = serialization.load_pem_private_key(
            f.read(), password=None, backend=default_backend()
        )

    # fake fxa uid
    fxa_uid = hashlib.sha256(email.encode()).hexdigest()[:32]

    # JWT payload
    now = int(time.time())
    payload = {
        "sub": fxa_uid,
        "scope": OAUTH_SCOPE,
        "fxa-generation": 0,
        "client_id": client_id,
        "iat": now,
        "exp": now + (12 * 3600),
        "iss": OAUTH_ISSUER,
    }

    # sign JWT
    oauth_token = jwt.encode(
        payload,
        private_key,
        algorithm=OAUTH_JWT_ALGORITHM,
        headers={"typ": "application/at+jwt"},
    )

    key_id = "1234-qqo"

    if os.environ.get("DEBUG_OAUTH"):
        print(f"DEBUG: Created self-signed JWT for: {email} / {fxa_uid}")

    return oauth_token, email, fxa_uid, key_id, None, None, None


def _is_oauth_token_expired(oauth_token: str, buffer_seconds: int = 300) -> bool:
    """Check if an OAuth token is expired or will expire soon.

    Args:
        oauth_token: JWT token to check.
        buffer_seconds: Consider token expired if it expires within this many seconds.

    Returns:
        bool: True if token is expired or will expire soon, False otherwise.

    """
    try:
        decoded = jwt.decode(oauth_token, options={"verify_signature": False})
        exp = decoded.get("exp")

        if exp is None:
            return False

        current_time = time.time()
        return current_time >= (exp - buffer_seconds)

    except Exception:
        return True


def _create_fxa_account() -> tuple[
    str, str, str, str, Optional[Any], Optional[OAuthClient], Optional[dict[str, Any]]
]:
    """Create account credentials.

    If OAUTH_PRIVATE_KEY_FILE is provided, creates self-signed JWT. Otherwise,
    creates real, testing account via FxA API.  The account is deleted at the
    end of the test.

    Returns:
        tuple: A tuple containing:
            - oauth_token: OAuth access token
            - acct_email: Account email address
            - fxa_uid: FxA uid
            - key_id: Key id
            - fxa_session: FxA session, or None for self-signed
            - oauth_client: OAuthClient instance, or None for self-signed
            - cleanup_info: Dict with cleanup info, or None for self-signed

    """
    if OAUTH_PRIVATE_KEY_FILE:
        email = f"molotov-{int(time.time())}-{random.randint(1000, 9999)}@example.com"
        return _create_self_signed_jwt(email)

    # use FxA to create account and fetch JWT
    acct = TestEmailAccount()
    client = Client(FXA_API_HOST)
    oauth_client = OAuthClient(CLIENT_ID, None, server_url=FXA_OAUTH_HOST)
    fxa_password = _generate_password()
    session = client.create_account(acct.email, password=fxa_password)

    # wait for account verification email
    max_retries = 20
    for _ in range(max_retries):
        if acct.messages:
            break
        time.sleep(0.5)
        acct.fetch()

    if not acct.messages:
        raise ValueError("Failed to receive FxA verification email")

    # verify account with code
    verified = False
    for m in acct.messages:
        if "x-verify-code" in m["headers"]:
            session.verify_email_code(m["headers"]["x-verify-code"])
            verified = True
            break

    if not verified:
        raise ValueError("Failed to find verification code in email")

    oauth_token = oauth_client.authorize_token(session, OAUTH_SCOPE)
    key_id = "1234-qqo"
    cleanup_info = {
        "client": client,
        "acct": acct,
        "email": acct.email,
        "password": fxa_password,
    }

    _track_account_creation(acct.email, fxa_password, session.uid)

    return (
        oauth_token,
        acct.email,
        session.uid,
        key_id,
        session,
        oauth_client,
        cleanup_info,
    )
