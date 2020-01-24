# User Migration Script

This is a workspace for testing user migration from the old databases to the new durable one.

Avro is a JSON like transport system. I'm not quite sure it's really needed. Mostly since
it's basically "read a row/write a row" only with a middleman system. I wonder if it might
be better to just iterate through mysql on a thread and write to spanner directly.

There are several candidate scrips that you can use. More than likely, you want to use

`migrate_user.py --dsns <file of database DSNs> --users <file of user ids>
    [--token_dsn <tokenserver DSN>]`

where:

* *dsns* - a file containing the mysql and spanner DSNs for the users. Each DSN should be on a single line. Currently only one DSN of a given type is permitted.

(e.g.)

```text
mysql://test:test@localhost/syncstorage
spanner://projects/sync-spanner-dev-225401/instances/spanner-test/databases/sync_schema3
```

* *users* - A file containing the list of mysql userIDs to move. Each should be on a new line.

(e.g.)

```text
1
3
1298127571
```

* *token_dsn* - An optional DSN to the Token Server DB. The script will automatically update the `users` table to indicate the user is now on the spanner node (`800`). If no *token_dsn* option is provided, the token_db is not altered, manual updates may be required.

The other scripts (e.g. `dump_mysql.py`) dump the node database into Arvo compatible format. Additional work may be required to format the data for eventual import. Note that these scripts may require python 2.7 due to dependencies in the Arvo library.

## installation

`virtualenv venv && venv/bin/pip install -r requirements.txt`

## running

Since you will be connecting to the GCP Spanner API, you will need to have set the `GOOGLE_APPLICATION_CREDENTIALS` env var before running these scripts.

`GOOGLE_APPLICATION_CREDENTIALS=path/to/creds.json venv/bin/python dump_avro.py`
