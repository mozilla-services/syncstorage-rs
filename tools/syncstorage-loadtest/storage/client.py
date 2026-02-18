"""HTTP client for SyncStorage API."""

import hashlib
import hmac
import os
import random
import time
from typing import Any, Optional
from urllib.parse import urlencode, urlparse, urlunparse

from molotov import json_request
from tokenlib import get_derived_secret as derive
from tokenlib import make_token

from .auth import (
    ASSERTION_LIFETIME,
    OAUTH_PRIVATE_KEY_FILE,
    OAUTH_SCOPE,
    _create_fxa_account,
    _create_self_signed_jwt,
    _is_oauth_token_expired,
    _remove_account_from_tracking,
)
from .utils import b64encode

_DEFAULT = os.environ.get("SERVER_URL", "https://token.stage.mozaws.net")


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
