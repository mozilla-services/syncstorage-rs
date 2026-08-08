# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Shared helpers for the Python utilities under ``tools/``.

This is a plain importable package: every ``tools/*`` project sets
``package-mode = false``, and the release image installs each tool from an
exported ``requirements.txt`` with ``pip install --no-index``, which cannot
resolve a local path dependency. Consumers import it by putting the repo root
(or ``tools/``) on the path (``pythonpath`` in ``pyproject.toml`` for pytest,
``PYTHONPATH`` for scripts).
"""
