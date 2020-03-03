#! venv/bin/python

# This file is historical.
# We're using `migrate_node.py`, however this file may be useful in the future
# if we determine there's a problem with directly transcribing the data from
# mysql to spanner.
#
# painfully stupid script to check out dumping mysql databases to avro.
# Avro is basically "JSON" for databases. It's not super complicated & it has
# issues.
#

import avro.schema
import argparse
import binascii
import csv
import base64
import math
import time
import os
import random
import re

from avro.datafile import DataFileWriter
from avro.io import DatumWriter
from mysql import connector
try:
    from urllib.parse import urlparse
except:
    from urlparse import urlparse


MAX_ROWS=1500000

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
        '--col_schema', default="user_collection.avsc",
        help="User Collection schema description"
    )
    parser.add_argument(
        '--output', default="output.avso",
        help="Output file")
    parser.add_argument(
        '--limit', type=int, default=1500000,
        help="Limit each read chunk to n rows")
    parser.add_argument(
        '--offset', type=int, default=0,
        help="UID to start at")
    parser.add_argument(
        '--deanon', action='store_false',
        dest='anon',
        help="Anonymize the user data"
    )
    parser.add_argument(
        '--start_bso', default=0,
        type=int,
        help="start dumping BSO database"
    )
    parser.add_argument(
        '--end_bso',
        type=int, default=19,
        help="last BSO database to dump"
    )
    parser.add_argument(
        '--token_file',
        default='users.csv',
        help="token user database dump CSV"
    )
    parser.add_argument(
        '--skip_collections', action='store_false',
        help="skip user_collections table"
    )

    return parser.parse_args()


def conf_db(dsn):
    dsn = urlparse(dsn)
    """
    if dsn.scheme != "mysql":
        raise BadDSNException("Invalid MySQL dsn: {}".format(dsn))
    """
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


user_ids = {}

def read_in_token_file(filename):
    global user_ids
    # you can generate the token file using
    # `mysql -e "select uid, email, generation, keys_changed_at, \
    #  client_state from users;" > users.csv`
    #
    # future opt: write the transmogrified file to either sqlite3
    # or static files.
    print("Processing token file...")
    with open(filename) as csv_file:
        for (uid, email, generation,
             keys_changed_at, client_state) in csv.reader(
                 csv_file, delimiter="\t"):
            if uid == 'uid':
                # skip the header row.
                continue
            fxa_uid = email.split('@')[0]
            fxa_kid = "{:013d}-{}".format(
                int(keys_changed_at or generation),
                base64.urlsafe_b64encode(
                    binascii.unhexlify(client_state)
                    ).rstrip(b'=').decode('ascii'))
            user_ids[uid] = (fxa_kid, fxa_uid)


def get_fxa_id(user_id, anon=True):
    global user_ids
    if user_id in user_ids:
        return user_ids[user_id]
    if anon:
        fxa_uid = binascii.hexlify(
            os.urandom(16)).decode('utf-8')
        fxa_kid = binascii.hexlify(
            os.urandom(16)).decode('utf-8')
        user_ids[user_id] = (fxa_kid, fxa_uid)
        return (fxa_kid, fxa_uid)


def dump_user_collections(schema, dsn, args):
    # userid => fxa_kid
    #           fxa_uid
    # collection => collection_id
    # last_modified => modified
    db = conf_db(dsn)
    cursor = db.cursor()
    out_file = args.output.rsplit('.', 1)
    out_file_name = "{}_user_collections.{}".format(
        out_file[0], out_file[1]
    )
    writer = DataFileWriter(
        open(out_file_name, "wb"), DatumWriter(), schema)
    sql = """
    SELECT userid, collection, last_modified from user_collections
    """
    start = time.time()
    try:
        cursor.execute(sql)
        row = 0
        for (user_id, collection_id, last_modified) in cursor:
            (fxa_uid, fxa_kid) = get_fxa_id(user_id, args.anon)
            try:
                writer.append({
                    "collection_id": collection_id,
                    "fxa_kid": fxa_kid,
                    "fxa_uid": fxa_uid,
                    "modified": last_modified
                })
            except Exception as ex:
                import pdb; pdb.set_trace()
                print (ex)
            row += 1
        print(
            "Dumped {} user_collection rows in {} seconds".format(
                row, time.time() - start
            ))
    finally:
        writer.close()
        cursor.close()


def dump_rows(bso_number, chunk_offset, db, writer, args):
    # bso column mapping:
    # id => bso_id
    # collection => collection_id
    # sortindex => sortindex
    # modified => modified
    # payload => payload
    # payload_size => NONE
    # ttl => expiry

    ivre = re.compile(r'("IV": ?"[^"]+")')
    print("Querying.... bso{} @{}".format(bso_number, chunk_offset))
    sql = """
    SELECT userid, collection, id,
    ttl, modified, payload,
    sortindex from bso{} LIMIT {} OFFSET {}""".format(
        bso_number, args.limit, chunk_offset)
    cursor = db.cursor()
    user = None
    row_count = 0
    try:
        cursor.execute(sql)
        print("Dumping...")
        for (userid, cid, bid, exp, mod, pay, si) in cursor:
            if args.anon:
                replacement = encode_bytes_b64(os.urandom(16))
                pay = ivre.sub('"IV":"{}"'.format(replacement), pay)
            if userid != user:
                (fxa_kid, fxa_uid) = get_fxa_id(userid, args.anon)
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
            row_count += 1
            if (chunk_offset + row_count) % 1000 == 0:
                print("BSO:{} Row: {}".format(bso_number, chunk_offset + row_count))
            if row_count >= MAX_ROWS:
                break
    except Exception as e:
        print("Deadline hit at: {} ({})".format(
            chunk_offset + row_count, e))
    finally:
        cursor.close()
    return row_count


def count_rows(db, bso_num=0):
    cursor = db.cursor()
    try:
        cursor.execute("SELECT Count(*) from bso{}".format(bso_num))
        return cursor.fetchone()[0]
    finally:
        cursor.close()


def dump_data(bso_number, schema, dsn, args):
    offset = args.offset or 0
    total_rows = 0
    # things time out around 1_500_000 rows.
    db = conf_db(dsn)
    out_file = args.output.rsplit('.', 1)
    row_count = count_rows(db, bso_number)
    for chunk in range(
        max(1, math.trunc(math.ceil(row_count / MAX_ROWS)))):
        print(
            "Dumping {} rows from bso#{} into chunk {}".format(
                row_count, bso_number, chunk))
        out_file_name = "{}_{}_{}.{}".format(
            out_file[0], bso_number, hex(chunk), out_file[1]
        )
        writer = DataFileWriter(
            open(out_file_name, "wb"), DatumWriter(), schema)
        rows = dump_rows(
            bso_number=bso_number,
            chunk_offset=offset,
            db=db,
            writer=writer,
            args=args)
        writer.close()
        if rows == 0:
            break
        offset = offset + rows
        chunk += 1
    return rows


def main():
    args = get_args()
    rows = 0
    dsns = open(args.dsns).readlines()
    schema = avro.schema.parse(open(args.schema, "rb").read())
    col_schema = avro.schema.parse(open(args.col_schema, "rb").read())
    if args.token_file:
        read_in_token_file(args.token_file)
    start = time.time()
    for dsn in dsns:
        print("Starting: {}".format(dsn))
        try:
            if not args.skip_collections:
                dump_user_collections(col_schema, dsn, args)
            for bso_num in range(args.start_bso, args.end_bso+1):
                rows = dump_data(bso_num, schema, dsn, args)
        except Exception as ex:
            print("Could not process {}: {}".format(dsn, ex))
    print("Dumped: {} rows in {} seconds".format(rows, time.time() - start))


if __name__ == "__main__":
    main()
