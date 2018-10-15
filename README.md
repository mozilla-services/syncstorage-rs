[![Build](https://travis-ci.org/mozilla-services/syncstorage-rs.svg?branch=master)](https://travis-ci.org/mozilla-services/syncstorage-rs)
[![License: MPL 2.0](https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg)](https://opensource.org/licenses/MPL-2.0)

# Syncstorage-rs

Mozilla Sync Storage node built with [Rust](https://rust-lang.org).

API docs: https://mozilla-services.readthedocs.io/en/latest/storage/apis-1.5.html

Code docs: https://mozilla-services.github.io/syncstorage-rs/syncstorage/

Functional tests live in https://github.com/mozilla-services/server-syncstorage/
and can be run against a local server like this:

```apple js
git clone https://github.com/mozilla-services/server-syncstorage/
cd server-syncstorage
make build
./local/bin/python syncstorage/tests/functional/test_storage.py http://localhost:8000
```
