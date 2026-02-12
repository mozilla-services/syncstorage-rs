#! /bin/bash -xe
# the Sync server URL. (Presumes a locally running server.)
# *NOTE* this will use the first arg, but will default to `localhost:8000`
HOST=${1:-http://localhost:8000}
# The sync shared secret.
SECRET=${2:-secret0}
SERVER_URL=${HOST}#${SECRET} poetry run molotov --processes 2 --workers 149 --verbose
