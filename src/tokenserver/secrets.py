import binascii
import hashlib
from tokenlib import HKDF

# Namespace prefix for HKDF "info" parameter.
HKDF_INFO_NODE_SECRET = b"services.mozilla.com/mozsvc/v1/node_secret/"


def derive_secrets(master_secrets, node):
    hkdf_params = {
        "salt": None,
        "info": HKDF_INFO_NODE_SECRET + node.encode("utf-8"),
        "hashmod": hashlib.sha256,
    }
    node_secrets = []
    for master_secret in master_secrets:
        # We want each hex-encoded derived secret to be the same
        # size as its (presumably hex-encoded) master secret.
        size = len(master_secret) // 2

        node_secret = HKDF(master_secret.encode("utf-8"), size=size,
                           **hkdf_params)
        node_secrets.append(binascii.b2a_hex(node_secret).decode())
    return node_secrets
