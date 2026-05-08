"""Mock FxA OAuth server for integration testing."""

import json
import os
from wsgiref.simple_server import make_server as _make_server


def _mock_oauth_verify(environ, start_response):
    try:
        length = int(environ.get("CONTENT_LENGTH") or 0)
        body = json.loads(environ["wsgi.input"].read(length))
        payload = json.loads(body["token"])
        status = "%d OK" % payload["status"]
        response_body = json.dumps(payload["body"]).encode()
    except Exception as exc:
        status = "400 Bad Request"
        response_body = json.dumps({"error": str(exc)}).encode()
    start_response(status, [("Content-Type", "application/json")])
    return [response_body]


def _mock_oauth_jwk(environ, start_response):
    # The PyFxA OAuth client makes a request to the FxA OAuth server for its
    # current public RSA key. While the client allows us to pass in a JWK to
    # prevent this request from happening, mocking the endpoint is simpler.
    response_body = json.dumps({"keys": [{"fake": "RSA key"}]}).encode()
    start_response("200 OK", [("Content-Type", "application/json")])
    return [response_body]


_ROUTES = {
    "/v1/verify": _mock_oauth_verify,
    "/v1/jwks": _mock_oauth_jwk,
}


def _app(environ, start_response):
    path = environ.get("PATH_INFO", "")
    handler = _ROUTES.get(path)
    if handler is None:
        start_response("404 Not Found", [("Content-Type", "application/json")])
        return [json.dumps({"error": "not found"}).encode()]
    return handler(environ, start_response)


def make_server(host, port):
    """Create and return a mock FxA OAuth WSGI server bound to host and port."""
    return _make_server(host, port, _app)


if __name__ == "__main__":
    host = os.environ.get("MOCK_FXA_SERVER_HOST", "localhost")
    port = os.environ.get("MOCK_FXA_SERVER_PORT", 6000)

    with make_server(host, int(port)) as httpd:
        print(f"Running mock FxA server on {host}:{port}")
        httpd.serve_forever()
