from wsgiref.simple_server import make_server as _make_server
from pyramid.config import Configurator
from pyramid.response import Response
from pyramid.view import view_config
import json
import os


@view_config(route_name='mock_oauth_verify', renderer='json')
def _mock_oauth_verify(request):
    body = json.loads(request.json_body['token'])

    return Response(json=body['body'], content_type='application/json',
                    status=body['status'])


# The PyFxA OAuth client makes a request to the FxA OAuth server for its
# current public RSA key. While the client allows us to pass in a JWK to
# prevent this request from happening, mocking the endpoint is simpler.
@view_config(route_name='mock_oauth_jwk', renderer='json')
def _mock_oauth_jwk(request):
    return {'keys': [{'fake': 'RSA key'}]}


@view_config(route_name='mock_verify', renderer='json')
def _mock_browserid_verify(request):
    body = json.loads(request.json_body['assertion'])

    return Response(json=body['body'], content_type='application/json',
                    status=body['status'])


# This endpoint is used by the legacy Tokenserver during startup. We mock it
# here so the unit tests can be run against the legacy Tokenserver.
@view_config(route_name='mock_config', renderer='json')
def _mock_config(request):
    return {
        "browserid": {
            "issuer": "api-accounts.stage.mozaws.net",
            "verificationUrl": "https://verifier.stage.mozaws.net/v2"
        },
        "contentUrl": "https://accounts.stage.mozaws.net"
    }


def make_server(host, port):
    with Configurator() as config:
        config.add_route('mock_oauth_verify', '/v1/verify')
        config.add_view(_mock_oauth_verify, route_name='mock_oauth_verify',
                        renderer='json')

        config.add_route('mock_oauth_jwk', '/v1/jwks')
        config.add_view(_mock_oauth_jwk, route_name='mock_oauth_jwk',
                        renderer='json')

        config.add_route('mock_browserid_verify', '/v2')
        config.add_view(_mock_browserid_verify,
                        route_name='mock_browserid_verify',
                        renderer='json')

        config.add_route('mock_config', '/config')
        config.add_view(_mock_config, route_name='mock_config',
                        renderer='json')
        app = config.make_wsgi_app()

    return _make_server(host, port, app)


if __name__ == '__main__':
    host = os.environ.get('MOCK_FXA_SERVER_HOST', 'localhost')
    port = os.environ.get('MOCK_FXA_SERVER_PORT', 6000)

    with make_server(host, int(port)) as httpd:
        print("Running mock FxA server on %s:%s" % (host, port))
        httpd.serve_forever()
