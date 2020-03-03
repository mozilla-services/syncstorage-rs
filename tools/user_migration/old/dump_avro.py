#! venv/bin/python

# painfully stupid script to check out dumping a spanner database to avro.
# Avro is basically "JSON" for databases. It's not super complicated & it has
# issues (one of which is that it requires Python2).
# test run Dumped 2770783 rows in 457.566066027 seconds and produced a
# roughly 6.5GB file.
#
# Spanner also has a Deadline issue where it will kill a db connection after
# so many minutes (5?). Might be better to just divvy things up into clusters
# and have threads handle transporting records over.
#

import avro.schema
import argparse
import time

from avro.datafile import DataFileWriter
from avro.io import DatumWriter
from google.cloud import spanner


def get_args():
    parser = argparse.ArgumentParser(description="dump spanner to arvo files")
    parser.add_argument(
        '--instance_id', default="spanner-test",
        help="Spanner instance name")
    parser.add_argument(
        '--database_id',  default="sync_schema3",
        help="Spanner database name")
    parser.add_argument(
        '--schema', default="sync.avsc",
        help="Database schema description")
    parser.add_argument(
        '--output', default="output.avso",
        help="Output file")
    parser.add_argument(
        '--limit', type=int, default=1500000,
        help="Limit to n rows")
    return parser.parse_args()


def conf_spanner(args):
    spanner_client = spanner.Client()
    instance = spanner_client.instance(args.instance_id)
    database = instance.database(args.database_id)
    return database


def dump_rows(offset, db, writer, args):
    print("Querying.... @{}".format(offset))
    sql = """
    SELECT collection_id, fxa_kid, fxa_uid, bso_id,
    UNIX_MICROS(expiry), UNIX_MICROS(modified), payload,
    sortindex from bsos LIMIT {} OFFSET {}""".format(args.limit, offset)
    try:
        with db.snapshot() as snapshot:
            result = snapshot.execute_sql(sql)
            print("Dumping...")
            for row in result:
                writer.append({
                    "collection_id": row[0],
                    "fxa_kid": row[1],
                    "fxa_uid": row[2],
                    "bso_id": row[3],
                    "expiry": row[4],
                    "modified": row[5],
                    "payload": row[6],
                    "sortindex": row[7]})
                offset += 1
                if offset % 1000 == 0:
                    print("Row: {}".format(offset))
            return offset
    except Exception as ex:
        print("Deadline hit at: {} ({})".format(offset, ex))
        return offset


def count_rows(db):
    with db.snapshot() as snapshot:
        result = snapshot.execute_sql("SELECT Count(*) from bsos")
        return result.one()[0]


def dump_data(args, schema):
    offset = 0
    # things time out around 1_500_000 rows.
    db = conf_spanner(args)
    writer = DataFileWriter(
        open(args.output, "wb"), DatumWriter(), schema)
    row_count = count_rows(db)
    print("Dumping {} rows".format(row_count))
    while offset < row_count:
        old_offset = offset
        offset = dump_rows(offset=offset, db=db, writer=writer, args=args)
        if offset == old_offset:
            break
    writer.close()
    return row_count


def main():
    start = time.time()
    args = get_args()
    schema = avro.schema.parse(open(args.schema, "rb").read())
    rows = dump_data(args, schema)
    print("Dumped: {} rows in {} seconds".format(rows, time.time() - start))


if __name__ == "__main__":
    main()
