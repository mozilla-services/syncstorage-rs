#! venv/bin/python

# painfully stupid script to check out dumping mysql databases to avro.
# Avro is basically "JSON" for databases. It's not super complicated & it has
# issues (one of which is that it requires Python2).
#
#

import avro.schema
import argparse
import base64
import time

from avro.datafile import DataFileWriter
from avro.io import DatumWriter
from mysql import connector
from urlparse import urlparse


class BadDSNException(Exception):
    pass


def get_args():
    parser = argparse.ArgumentParser(description="dump spanner to arvo files")
    parser.add_argument(
        '--dsns', default="dsns.lst",
        help="file of new line separated DSNs")
    parser.add_argument(
        '--schema', default="sync.avsc",
        help="Database schema description")
    parser.add_argument(
        '--output', default="output.avso",
        help="Output file")
    parser.add_argument(
        '--limit', type=int, default=1500000,
        help="Limit each read chunk to n rows")
    return parser.parse_args()


def conf_db(dsn):
    dsn = urlparse(dsn)
    if dsn.scheme != "mysql":
        raise BadDSNException("Invalid MySQL dsn: {}".format(dsn))
    connection = connector.connect(
        user=dsn.username,
        password=dsn.password,
        host=dsn.hostname,
        port=dsn.port or 3306,
        database=dsn.path[1:]
    )
    return connection


# The following two functions are taken from browserid.utils
def encode_bytes_b64(value):
    return base64.urlsafe_b64encode(value).rstrip(b'=').decode('ascii')


def format_key_id(keys_changed_at, key_hash):
    return "{:013d}-{}".format(
        keys_changed_at,
        encode_bytes_b64(key_hash),
    )


def get_fxa_id(database, user):
    sql = """
        SELECT
            email, generation, keys_changed_at, client_state
        FROM users
            WHERE uid = {uid}
    """.format(uid=user)
    cursor = database.cursor()
    cursor.execute(sql)
    (email, generation, keys_changed_at, client_state) = cursor.next()
    fxa_uid = email.split('@')[0]
    fxa_kid = format_key_id(
        keys_changed_at or generation,
        bytes.fromhex(client_state),
    )
    cursor.close()
    return (fxa_kid, fxa_uid)


def dump_rows(offset, db, writer, args):
    # bso column mapping:
    # id => bso_id
    # collection => collection_id
    # sortindex => sortindex
    # modified => modified
    # payload => payload
    # payload_size => NONE
    # ttl => expiry

    print("Querying.... @{}".format(offset))
    sql = """
    SELECT userid, collection, id,
    ttl, modified, payload,
    sortindex from bso LIMIT {} OFFSET {}""".format(
        args.limit, offset)
    cursor = db.cursor()
    user = None
    try:
        cursor.execute(sql)
        print("Dumping...")
        for (userid, cid, bid, exp, mod, pay, si) in cursor:
            if userid != user:
                (fxa_kid, fxa_uid) = get_fxa_id(db, userid)
                user = userid
            writer.append({
                "collection_id": cid,
                "fxa_kid": fxa_kid,
                "fxa_uid": fxa_uid,
                "bso_id": bid,
                "expiry": exp,
                "modified": mod,
                "payload": pay,
                "sortindex": si})
            offset += 1
            if offset % 1000 == 0:
                print("Row: {}".format(offset))
        return offset
    except Exception as e:
        print("Deadline hit at: {} ({})".format(offset, e))
        return offset
    finally:
        cursor.close()


def count_rows(db):
    cursor = db.cursor()
    try:
        cursor.execute("SELECT Count(*) from bso")
        return cursor.fetchone()[0]
    finally:
        cursor.close()


def dump_data(args, schema, dsn):
    offset = 0
    # things time out around 1_500_000 rows.
    # yes, this dumps from spanner for now, I needed a big db to query
    db = conf_db(dsn)
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
    dsns = open(args.dsns).readlines()
    schema = avro.schema.parse(open(args.schema, "rb").read())
    for dsn in dsns:
        print("Starting: {}".format(dsn))
        try:
            rows = dump_data(args, schema, dsn)
        except Exception as ex:
            print("Could not process {}: {}".format(dsn, ex))
    print("Dumped: {} rows in {} seconds".format(rows, time.time() - start))


if __name__ == "__main__":
    main()
