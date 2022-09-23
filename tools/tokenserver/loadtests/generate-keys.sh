#!/bin/sh

# Generate a private RSA key
openssl genrsa -out load_test.pem 2048

# Derive the public key from the private key
openssl rsa -in load_test.pem -pubout > load_test.pub

# Derive and print the JWK from the public key
python3 get_jwk.py load_test.pub > jwk.json
rm load_test.pub
