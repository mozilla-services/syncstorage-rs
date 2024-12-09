import sys

from authlib.jose import JsonWebKey

raw_public_key = open(sys.argv[1], "rb").read()
public_key = JsonWebKey.import_key(raw_public_key, {"kty": "RSA"})
print(public_key.as_json())
