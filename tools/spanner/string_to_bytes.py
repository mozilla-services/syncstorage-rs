import base64
import os
import sys

from google.cloud import spanner
from google.cloud.spanner_admin_database_v1.types import spanner_database_admin

OPERATION_TIMEOUT_SECONDS = 3
ONE_MB = 1024 * 1024
TEN_MB = 10 * ONE_MB

def create_database(client, instance_id, database_id):
    """Creates a database and tables for sample data."""
    database_admin_api = client.database_admin_api

    request = spanner_database_admin.CreateDatabaseRequest(
        parent=database_admin_api.instance_path(client.project, instance_id),
        create_statement=f"CREATE DATABASE `{database_id}`",
        extra_statements=[
            """CREATE TABLE Test (
            uid             STRING(MAX),
            payload         STRING(MAX),
            payload_bytes   BYTES(MAX),
        ) PRIMARY KEY (uid)""",
        ],
    )

    operation = database_admin_api.create_database(request=request)

    print("Waiting for operation to complete...")
    database = operation.result(OPERATION_TIMEOUT_SECONDS)

    print(
        "Created database {} on instance {}".format(
            database.name,
            database_admin_api.instance_path(client.project, instance_id),
        )
    )


def store_payload0(database):
    def insert(transaction):
        row_ct = transaction.execute_update(
            "INSERT Test (uid, payload, payload_bytes) "
            " VALUES ('1', '日本国', b'日本国')"
        )
        print("{} record(s) inserted.".format(row_ct))

    database.run_in_transaction(insert)


def store_payload(database):
    """
>>> "日本国".encode('utf-8')
b'\xe6\x97\xa5\xe6\x9c\xac\xe5\x9b\xbd'
>>> base64.b64encode("日本国".encode('utf-8'))
b'5pel5pys5Zu9'
"""
    with database.batch() as batch:
        batch.insert(
            table="Test",
            columns=("uid", "payload", "payload_bytes"),
            values=[
                (1, "日本国", b"5pel5pys5Zu9"),
                #google.api_core.exceptions.FailedPrecondition: 400 Could not parse 日本国 as BYTES. Byte strings must be base64-encoded. For more details, see https://cloud.google.com/spanner/docs/reference/rpc/google.spanner.v1#typecode
                #(2, "日本国", "日本国".encode("utf-8")),
            ],
        )

    print("Inserted data.")


def store_large_payload(database, size=TEN_MB):
    b = os.urandom(size)
    b64 = base64.b64encode(b)
    print(f"{len(b)} {len(b64)}")
    with database.batch() as batch:
        batch.insert(
            table="Test",
            columns=("uid", "payload", "payload_bytes"),
            values=[
                (2, "日本国", b64),
            ],
        )

    print("Inserted data.")


def get_payload(database):
    def get(transaction):
        # Define a SELECT query.
        query = """SELECT * FROM Test"""
        result = transaction.execute_sql(query)
        for row in result:
            print(row)
    database.run_in_transaction(get)


def get_payload_len(database):
    def get(transaction):
        """
>>> len("日本国")
3
>>> len("日本国".encode('utf-8'))
9
>>> len(base64.b64encode("日本国".encode('utf-8')))
12
"""
        query = """SELECT LENGTH(payload), LENGTH(payload_bytes) FROM Test"""
        result = transaction.execute_sql(query)
        for row in result:
            print(row)
    database.run_in_transaction(get)


def alter_string_to_bytes(client, instance_id, database_id):
    database_admin_api = client.database_admin_api

    request = spanner_database_admin.UpdateDatabaseDdlRequest(
        database=database_admin_api.database_path(
            client.project, instance_id, database_id
        ),
        statements=[
            "ALTER TABLE Test ALTER COLUMN payload BYTES(MAX)",
        ],
    )

    operation = database_admin_api.update_database_ddl(request)

    print("Waiting for ALTER operation to complete...")
    operation.result(OPERATION_TIMEOUT_SECONDS)
    print("ALTER completed.")


def main():
    instance_id = "test-instance"
    database_id = "test-database"
    client = spanner.Client(project="test-project")
    instance = client.instance(instance_id)
    database = instance.database(database_id)
    if "large" in sys.argv:
        create_database(client, instance_id, database_id)
        store_large_payload(database)
        get_payload_len(database)
    elif "too-large" in sys.argv:
        create_database(client, instance_id, database_id)
        """
google.api_core.exceptions.FailedPrecondition: 400 New value exceeds the maximum size limit for this column: Test.payload_bytes, size: 10485761, limit: 10485760.
        """
        store_large_payload(database, TEN_MB + 1)
        get_payload_len(database)
    else:
        create_database(client, instance_id, database_id)
        store_payload(database)
        get_payload(database)
        get_payload_len(database)
        alter_string_to_bytes(client, instance_id, database_id)
        get_payload(database)
        get_payload_len(database)


if __name__ == "__main__":
    main()
