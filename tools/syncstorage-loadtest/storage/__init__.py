"""Storage client for SyncStorage load testing with FxA authentication."""

import base64
import hashlib
import hmac
import json
import os
import random
import string
import time
from pathlib import Path
from typing import Any, Optional
from urllib.parse import urlencode, urlparse, urlunparse

import jwt
from cryptography.hazmat.backends import default_backend
from cryptography.hazmat.primitives import serialization
from fxa.core import Client
from fxa.oauth import Client as OAuthClient
from fxa.tests.utils import TestEmailAccount
from molotov import json_request
from tokenlib import get_derived_secret as derive
from tokenlib import make_token

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

_DEFAULT = os.environ.get("SERVER_URL", "https://token.stage.mozaws.net")
_ACCT_TRACKING_FILE = Path(__file__).parent.parent / ".accounts_tracking.json"


def b64encode(data: bytes) -> str:
    """Encode bytes to base64 ASCII string.

    Args:
        data: Bytes to encode.

    Returns:
        str: Base64-encoded ASCII string.

    """
    return base64.b64encode(data).decode("ascii")


def _generate_password() -> str:
    return "".join(random.choice(string.printable) for i in range(PASSWORD_LENGTH))


def _track_account_creation(email: str, password: str, fxa_uid: str) -> None:
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


class StorageClient(object):
    """Client for interacting with SyncStorage API with FxA authentication.

    Manages OAuth tokens, Hawk authentication, and HTTP requests to the
    SyncStorage service. Handles token expiration and renewal automatically.

    """

    def __init__(self, session, server_url=_DEFAULT):
        """Initialize the StorageClient.

        Args:
            session: The aiohttp session to use for requests.
            server_url: The server URL to connect to (default: from SERVER_URL env).

        """
        self.session = session
        self.timeskew = 0
        self.server_url = server_url
        self.uid = None
        self.auth_token = None
        self.auth_secret = None
        self.auth_expires_at = 0
        self.auth_regeneration_flag = False
        self.endpoint_url = None
        self.endpoint_scheme = None
        self.endpoint_host = None
        self.fxa_oauth_token = None
        self.fxa_uid = None
        self.fxa_key_id = None
        self.fxa_session = None  # For getting new OAuth tokens
        self.fxa_oauth_client = None  # For getting new OAuth tokens
        self.fxa_cleanup_info = None  # For account deletion
        self.generate()

    def _get_url(self, path: str, params: Optional[dict[str, str]] = None) -> str:
        url = self.endpoint_url + path
        if params is not None:
            url += "?" + urlencode(params)
        return url

    def __repr__(self):
        """Return string representation of the client.

        Returns:
            str: The authentication token as a string.

        """
        return str(self.auth_token)

    def generate(self) -> None:
        """Pick an identity, log in and generate the auth token.

        For OAuth: Creates FxA account once and caches credentials.

        """
        url = urlparse(self.server_url)

        if url.fragment:
            self.uid = random.randint(1, 1000000)
        else:
            if self.fxa_oauth_token is None:
                (
                    oauth_token,
                    acct_email,
                    fxa_uid,
                    key_id,
                    fxa_session,
                    oauth_client,
                    cleanup_info,
                ) = _create_fxa_account()
                self.fxa_oauth_token = oauth_token
                self.fxa_uid = fxa_uid
                self.fxa_key_id = key_id
                self.fxa_session = fxa_session
                self.fxa_oauth_client = oauth_client
                self.fxa_cleanup_info = cleanup_info
                if os.environ.get("DEBUG_OAUTH"):
                    print(f"DEBUG: Created FxA account: {acct_email} / {fxa_uid}")

        self.regenerate()

    def regenerate(self) -> None:
        """Generate an auth token for the selected identity."""
        # If the server_url has a hash fragment, it's a storage node and
        # that's the secret.  Otherwise it's a token server url.
        url = urlparse(self.server_url)

        if url.fragment:
            uid = self.uid
            endpoint = url._replace(
                path=url.path.rstrip("/") + "/1.5/" + str(uid),
                fragment="",
            )
            self.endpoint_url = urlunparse(endpoint)
            token_duration = ASSERTION_LIFETIME
            # Some storage backends use the numeric tokenserver uid, and some use
            # the raw fxa uid and kid.  Let's include mock values for both cases,
            # with everything derived from the mock uid for consistency..
            data = {
                "uid": uid,
                "fxa_uid": hashlib.sha256(
                    "{}:fxa_uid".format(uid).encode("ascii")
                ).hexdigest(),
                "fxa_kid": hashlib.sha256(
                    "{}:fxa_kid".format(uid).encode("ascii")
                ).hexdigest()[:32],
                "hashed_fxa_uid": hashlib.sha256(
                    "{}:hashed_fxa_uid".format(uid).encode("ascii")
                ).hexdigest(),
                "node": urlunparse(url._replace(path="", fragment="")),
                "expires": time.time() + token_duration,
            }
            auth_token = make_token(data, secret=url.fragment)
            self.auth_token = auth_token.encode("ascii")
            self.auth_secret = derive(auth_token, secret=url.fragment).encode("ascii")
            self.auth_expires_at = data["expires"]
        else:
            token_url = self.server_url + "/1.0/sync/1.5"

            if _is_oauth_token_expired(self.fxa_oauth_token):
                if os.environ.get("DEBUG_OAUTH"):
                    print(
                        f"DEBUG: OAuth token expired, fetching another for account: {self.fxa_uid}"
                    )

                # handle self-signed but really should just set a longer TTL?
                if OAUTH_PRIVATE_KEY_FILE:
                    email = f"loadtest-{self.fxa_uid}@example.com"
                    new_oauth_token, _, _, _, _, _, _ = _create_self_signed_jwt(email)
                    self.fxa_oauth_token = new_oauth_token
                else:
                    new_oauth_token = self.fxa_oauth_client.authorize_token(
                        self.fxa_session, OAUTH_SCOPE
                    )
                    self.fxa_oauth_token = new_oauth_token

                if os.environ.get("DEBUG_OAUTH"):
                    print(f"DEBUG: fetched new token for account: {self.fxa_uid}")

            oauth_token = self.fxa_oauth_token
            fxa_uid = self.fxa_uid
            key_id = self.fxa_key_id

            if os.environ.get("DEBUG_OAUTH"):
                print(f"DEBUG: Using existing OAuth token for account: {fxa_uid}")

            response = json_request(
                token_url,
                headers={
                    "Authorization": "Bearer {}".format(oauth_token),
                    "X-KeyID": key_id,
                },
            )

            if response is None:
                raise ValueError(
                    "Failed to get response from token server at {}".format(token_url)
                )

            if response.get("status") > 299:
                error_msg = "Request with OAuth token failed with status {}: {}".format(
                    response.get("status"), response
                )
                raise ValueError(error_msg)

            credentials = response.get("content")
            if credentials is None:
                raise ValueError(
                    "Token response missing content. Response: {}".format(response)
                )

            self.auth_token = credentials["id"].encode("ascii")
            self.auth_secret = credentials["key"].encode("ascii")
            self.endpoint_url = credentials["api_endpoint"]
            token_duration = credentials["duration"]

        # Regenerate tokens when they're close to expiring
        # but before they actually expire, to avoid spurious 401s.

        self.auth_expires_at = time.time() + (token_duration * 0.5)

        url = urlparse(self.endpoint_url)
        self.endpoint_scheme = url.scheme
        self.endpoint_path = url.path
        self.host_header = url.netloc
        if ":" in url.netloc:
            self.endpoint_host, self.endpoint_port = url.netloc.rsplit(":", 1)
        else:
            self.endpoint_host = url.netloc
            if url.scheme == "http":
                self.endpoint_port = "80"
            else:
                self.endpoint_port = "443"

    def _normalize(self, params: dict[str, str], url: str, meth: str) -> str:
        bits = []
        bits.append("hawk.1.header")
        bits.append(params["ts"])
        bits.append(params["nonce"])
        bits.append(meth)
        parsed_url = urlparse(url)
        if parsed_url.query:
            path_qs = parsed_url.path + "?" + parsed_url.query
        else:
            path_qs = parsed_url.path
        bits.append(path_qs)
        bits.append(self.endpoint_host.lower())
        bits.append(self.endpoint_port)
        bits.append(params.get("hash", ""))
        bits.append(params.get("ext", ""))
        bits.append("")  # to get the trailing newline
        return "\n".join(bits)

    def _sign(self, params: dict[str, str], url: str, meth: str) -> str:
        sigstr = self._normalize(params, url, meth)
        sigstr_bytes = sigstr.encode("ascii")
        key = self.auth_secret
        hashmod = hashlib.sha256
        return b64encode(hmac.new(key, sigstr_bytes, hashmod).digest())

    def _auth(self, meth: str, url: str) -> str:
        ts = time.time()
        if ts >= self.auth_expires_at:
            # Try to exclude multiple co-routines from regenerating
            # the token.  It's safe to regenerate multiple times
            # but would be wasted work.
            if not self.auth_regeneration_flag:
                self.auth_regeneration_flag = True
                try:
                    self.regenerate()
                finally:
                    self.auth_regeneration_flag = False
        params: dict[str, str] = {}
        params["id"] = self.auth_token.decode("ascii")
        params["ts"] = str(int(ts) + self.timeskew)
        params["nonce"] = b64encode(os.urandom(5))
        params["mac"] = self._sign(params, url, meth)
        res = ", ".join(['%s="%s"' % (k, v) for k, v in params.items()])
        return "Hawk " + res

    async def _retry(
        self,
        meth: str,
        path_qs: str,
        params: Optional[dict[str, str]],
        data: Optional[str],
        statuses: Optional[tuple[int, ...]] = None,
    ) -> tuple[Any, Any]:
        url = self._get_url(path_qs, params)
        headers = {
            "Authorization": self._auth(meth, url),
            "Host": self.host_header,
            "Content-Type": "application/json",
            "X-Confirm-Delete": "1",
        }

        call = getattr(self.session, meth.lower())
        options: dict[str, Any] = {"headers": headers}
        if meth.lower() in ("post", "put"):
            options["data"] = data

        async with call(url, **options) as resp:
            if resp.status == 401:
                server_time = int(float(resp.headers["X-Weave-Timestamp"]))
                self.timeskew = server_time - int(time.time())
                options["headers"]["Authorization"] = self._auth(meth, url)
                async with call(url, **options) as resp:
                    if statuses is not None:
                        assert resp.status in statuses, (
                            "Reauth Response {} not in {}".format(resp.status, statuses)
                        )
                    body = await resp.json()
                    return resp, body
            else:
                if statuses is not None:
                    assert resp.status in statuses, "Response {} not in {}".format(
                        resp.status, statuses
                    )

                body = await resp.json()
                return resp, body

    async def post(
        self,
        path_qs: str,
        data: Optional[str] = None,
        statuses: Optional[tuple[int, ...]] = None,
        params: Optional[dict[str, str]] = None,
    ) -> tuple[Any, Any]:
        """Send a POST request to the storage server.

        Args:
            path_qs: The path for the request.
            data: Optional data payload to send.
            statuses: Optional tuple of acceptable HTTP status codes.
            params: Optional query parameters.

        Returns:
            tuple: Response object and parsed JSON body.

        """
        return await self._retry("POST", path_qs, params, data, statuses)

    async def put(
        self,
        path_qs: str,
        data: Optional[str] = None,
        statuses: Optional[tuple[int, ...]] = None,
        params: Optional[dict[str, str]] = None,
    ) -> tuple[Any, Any]:
        """Send a PUT request to the storage server.

        Args:
            path_qs: The path for the request.
            data: Optional data payload to send.
            statuses: Optional tuple of acceptable HTTP status codes.
            params: Optional query parameters.

        Returns:
            tuple: Response object and parsed JSON body.

        """
        return await self._retry("PUT", path_qs, params, data, statuses)

    async def get(
        self,
        path_qs: str,
        statuses: Optional[tuple[int, ...]] = None,
        params: Optional[dict[str, str]] = None,
    ) -> tuple[Any, Any]:
        """Send a GET request to the storage server.

        Args:
            path_qs: The path for the request.
            statuses: Optional tuple of acceptable HTTP status codes.
            params: Optional query parameters.

        Returns:
            tuple: Response object and parsed JSON body.

        """
        return await self._retry("GET", path_qs, params, data=None, statuses=statuses)

    async def delete(
        self,
        path_qs: str,
        data: Optional[str] = None,
        statuses: Optional[tuple[int, ...]] = None,
        params: Optional[dict[str, str]] = None,
    ) -> tuple[Any, Any]:
        """Send a DELETE request to the storage server.

        Args:
            path_qs: The path for the request.
            data: Optional data payload to send.
            statuses: Optional tuple of acceptable HTTP status codes.
            params: Optional query parameters.

        Returns:
            tuple: Response object and parsed JSON body.

        """
        return await self._retry("DELETE", path_qs, params, data, statuses)

    def cleanup(self) -> None:
        """Clean up the FxA account created for this test session."""
        if self.fxa_cleanup_info is None:
            return

        try:
            self.fxa_cleanup_info["acct"].clear()

            client = self.fxa_cleanup_info["client"]
            email = self.fxa_cleanup_info["email"]
            password = self.fxa_cleanup_info["password"]

            client.destroy_account(email, password)
            _remove_account_from_tracking(email)

            if os.environ.get("DEBUG_OAUTH"):
                print(f"DEBUG: Deleted FxA account: {email} / {self.fxa_uid}")

        except Exception as ex:
            print(
                f"Warning: Encountered error when deleting FxA account {email} / {self.fxa_uid}: {ex}"
            )
