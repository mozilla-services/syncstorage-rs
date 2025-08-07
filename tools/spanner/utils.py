# Utility Module for spanner CLI scripts
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

import os
from enum import auto, Enum
from urllib import parse
from typing import Tuple

DSN_URL = "SYNC_SYNCSTORAGE__DATABASE_URL"
"""
Environment variable that stores Sync database URL
Depending on deployment, can be MySQL or Spanner.
In this context, should always point to spanner for these scripts.
"""


class Mode(Enum):
    URL = auto()
    ENV_VAR = auto()


def ids_from_env(dsn=DSN_URL, mode=Mode.ENV_VAR) -> Tuple[str, str, str]:
    """
    Function that extracts the instance, project, and database ids from the DSN url.
    It is defined as the SYNC_SYNCSTORAGE__DATABASE_URL environment variable.
    The defined defaults are in webservices-infra/sync and can be configured there for
    production runs.

    `dsn` argument is set to default to the `DSN_URL` constant.

    For reference, an example spanner url passed in is in the following format:

    `spanner://projects/moz-fx-sync-prod-xxxx/instances/sync/databases/syncdb`
    database_id = `syncdb`, instance_id = `sync`, project_id = `moz-fx-sync-prod-xxxx`
    """
    # Change these to reflect your Spanner instance install
    instance_id = None
    database_id = None
    project_id = None

    try:
        if mode == Mode.ENV_VAR:
            url = os.environ.get(dsn)
            if not url:
                raise Exception(f"No env var found for provided DSN: {dsn}")
        elif mode == Mode.URL:
            url = dsn
            if not url:
                raise Exception(f"No valid url found: {url}")
        parsed_url = parse.urlparse(url)
        if parsed_url.scheme == "spanner":
            path = parsed_url.path.split("/")
            instance_id = path[-3]
            project_id = path[-5]
            database_id = path[-1]
    except Exception as e:
        print(f"Exception parsing url: {e}")
    # Fallbacks if not set
    if not instance_id:
        instance_id = os.environ.get("INSTANCE_ID", "spanner-test")
    if not database_id:
        database_id = os.environ.get("DATABASE_ID", "sync_stage")
    if not project_id:
        project_id = os.environ.get("GOOGLE_CLOUD_PROJECT", "test-project")

    return (instance_id, database_id, project_id)
