#!/usr/bin/env python3
"""Create a Hawk token for tests

requires hawkauthlib, tokenlib, webob

"""
import hmac
import time
from datetime import timedelta
from hashlib import sha256

import hawkauthlib
import tokenlib
from webob.request import Request

LEGACY_UID = 1
FXA_UID = "319b98f9961ff1dbdd07313cd6ba925a"
FXA_KID = "XXX"
DEVICE_ID = "XXX"
NODE = "http://localhost:5000"
# 10 years
DURATION = timedelta(days=10 * 365).total_seconds()

SECRET = "Ted Koppel is a robot"
HMAC_KEY = b"foo"


def create_token():
    expires = int(time.time()) + DURATION
    token_data = {
        'uid': LEGACY_UID,
        'node': NODE,
        'expires': expires,
        'fxa_uid': FXA_UID,
        'fxa_kid': FXA_KID,
        'hashed_fxa_uid': metrics_hash(FXA_UID),
        'hashed_device_id': metrics_hash(DEVICE_ID),
    }
    token = tokenlib.make_token(token_data, secret=SECRET)
    key = tokenlib.get_derived_secret(token, secret=SECRET)
    return token, key, expires


def metrics_hash(value):
    hasher = hmac.new(HMAC_KEY, b'', sha256)
    # value may be an email address, in which case we only want the first part
    hasher.update(value.encode('utf-8').split(b"@", 1)[0])
    return hasher.hexdigest()

def main():
    token, key, expires = create_token()
    req = Request.blank("https://localhost:5000/storage/1.5/1/storage/col2")
    header = hawkauthlib.sign_request(req, token, key)
    print("Hawk Authorization Header: ", header)
    print("Expires: ", expires)

if __name__ == '__main__':
    main()
