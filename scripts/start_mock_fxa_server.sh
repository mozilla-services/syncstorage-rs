#!/bin/sh

. /app/venv/bin/activate
python3 /app/tools/integration_tests/tokenserver/mock_fxa_server.py

