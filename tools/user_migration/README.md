 # User Migration Script

This is a workspace for testing user migration from the old databases to the new durable one.

There are several candidate scrips that you can use. More than likely, you want to use

```bash
GOOGLE_APPLICATION_CREDENTIALS=credentials.json migrate_node.py \
    [--dsns=move_dsns.lst] \
    [--deanon --fxa_file=users.csv] \
    [--start_bso=0] \
    [--end_bso=19]
```
where:

* *dsns* - a file containing the mysql and spanner DSNs for the users. Each DSN should be on a single line. Currently only one DSN of a given type is permitted.

(e.g.)

```text
mysql://test:test@localhost/syncstorage
spanner://projects/sync-spanner-dev-225401/instances/spanner-test/databases/sync_schema3
```

* *users.csv* - a mysql dump of the token database. This file is only needed if the `--deanon` de-anonymization flag is set. By default, data is anononymized to prevent accidental movement.
You can produce this file from the following:
```bash
mysql -e "select uid, email, generation, keys_changed_at, \
 client_state from users;" > users.csv`
```
The script will automatically skip the title row, and presumes that fields are tab separated.

UserIDs are converted to fxa_uid/fxa_kid values and cached locally.

## installation

```bash
virtualenv venv && venv/bin/pip install -r requirements.txt
```

## running

Since you will be connecting to the GCP Spanner API, you will need to have set the `GOOGLE_APPLICATION_CREDENTIALS` env var before running these scripts. This environment variable should point to the exported Google Credentials acquired from the GCP console.

The script will take the following actions:

1. fetch all users from a given node.
1. compare and port all user_collections over (NOTE: this may involve remapping collecitonid values.)
1. begin copying over user information from mysql to spanner

Overall performance may be improved by "batching" BSOs to different
processes using:

`--start_bso` the BSO database (defaults to 0, inclusive) to begin
copying from

`--end_bso` the final BSO databse (defaults to 19, inclusive) to copy
from

Note that these are inclusive values. So to split between two
processes, you would want to use

```bash
migrate_node.py --start_bso=0 --end_bso=9 &
migrate_node.py --start_bso=10 --end_bso=19 &
```

(As short hand for this case, you could also do:
```
migrate_node.py --end_bso=9 &
migrate_node.py --start_bso=10 &
```
and let the defaults handle the rest.)
