import argparse
import random
import time
import sys
import uuid
import os
import signal

from urllib.parse import quote_plus

def config():
    parser = argparse.ArgumentParser(
        description="Generate quota busting amounts of bookmarks"
    )
    parser.add_argument(
        "-o", "--output",
        default="./quota_bookmarks.html",
        help="output file [./quota_bookmarks.html]"
    )
    parser.add_argument(
        "-c", "--count",
        default=8000000,
        help="Number of bookmarks to generate [8,000,000]"
    )
    parser.add_argument(
        "--max_size",
        default=0,
        help="max number of gigabytes to generate"
    )
    parser.add_argument(
        "--start_date",
        default=int(time.time()) - 31556952 * 3,
        help="when to start making fake bookmarks [3 years ago]"
    )
    parser.add_argument(
        "--words",
        default="words.lst",
        help="random word list [words.lst]"
    )
    return parser.parse_args()

def gen_url(args, words):
    domains = ["example.com", "example.org", "example.net", "example.mil",
               "example.edu", "example.gov", "example.uk", "example.ie",
               "example.au", "evilonastick.com"]
    path_bits = random.sample(words, random.randrange(10))
    url = "https://{domain}/{buster}".format(
        domain = random.choice(domains),
        buster = uuid.uuid4().hex
    )
    for i in range(0,random.randint(0, len(path_bits))):
        try:
            bit = quote_plus(random.choice(path_bits))
            url = "{}/{}".format(url, bit)
        except TypeError:
            pass
    return url

def main(args, words):

    now = int(time.time())
    if args.output == "-":
        output = sys.stdout
    else:
        output = open(args.output, "w")
    output.write("""<!DOCTYPE NETSCAPE-Bookmark-file-1>
<title>Bookmarks</title>
<h1>Bookmarks</h1>
<DL>
"""
)
    def close_file(output):
        output.write("</DL><p>")
        output.close()
        sys.exit()

    def signal_handler(sig, frame):
        print("Aborting, closing file.")
        close_file(output)

    signal.signal(signal.SIGINT, signal_handler)
    for i in range(0, int(args.count)):
        if not i % 10000 and i > 0:
            size = os.stat(args.output).st_size / 1000000000
            if args.max_size and size > args.max_size:
                print("Max size achieved")
                close_file(output)
            print("| {: 7d} rows, {:0.3f} GB".format(
                i, size))
        elif not i % 1000:
            print(".", end="")
        url = gen_url(args, words)
        add_date = random.randint(args.start_date, now)
        visit_date = random.randint(add_date, now)
        mod_date = random.randint(add_date, visit_date)
        tags = ','.join(random.sample(words, 1+random.randrange(8)))
        title = ' '.join(random.sample(words, 1+random.randrange(12))).capitalize()
        output.write(
            """<DL><A HREF="{url}" ADD_DATE="{add_date}" """
            """LAST_VISIT="{visit_date}" LAST_MODIFIED="{mod_date}" """
            """TAGS="quota,{tags}">{title}</a>\n""".format(
                url=url,
                add_date=add_date,
                visit_date=visit_date,
                mod_date=mod_date,
                tags=tags,
                title=title,
            ))
    output.write("""</DL><p>""")

args = config()
words = open(args.words).read().splitlines()
main(args, words)