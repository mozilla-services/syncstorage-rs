import functions_framework
import json

from flask import abort, Response


# GCP doesn't allow us to define multiple routes in a single Cloud Function,
# so we handle routing here.
@functions_framework.http
def mock_fxa_server(request):
    if request.path == '/v1/verify':
        body = json.loads(request.json['token'])
        response = json.dumps(body['body'])

        return Response(response=response, content_type='application/json',
                        status=body['status'])
    elif request.path == '/v1/jwks':
        return {'keys': [{'fake': 'RSA key'}]}
    else:
        abort(404)
