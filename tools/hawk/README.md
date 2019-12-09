# Make a Hawk compatible Auth header

1) The best way to install this is probably to set up a python virtual
env.

`python3 -m venv venv && venv/bin/pip install -r requirements.txt`

this will create a python virtual environment in the `/venv` directory.

*Note* You may need to install `python3-venv` for the above to work.

Once the virtual env is installed, run `. venv/bin/activate`. This
will ensure that calls to python and python tools happen within this
virutal environment.

2) install the requirements using:

`venv/bin/pip install -r requirements.txt`

3) To create a Token Header:

You'll need to pass along your `SYNC_MASTER_SECRET` and the uri you'll be testing in order to generate a valid Hawk Id:

`venv/bin/python make_hawk_token.py --uri /1.5/1/storage/meta/global --secret=$SYNC_MASTER_SECRET --as_header`

** For testing against uri's using methods other than GET, you'll need to pass along the `--method` flag to generate your token. Ie, `venv/bin/python make_hawk_token.py --method PUT --uri /1.5/1/storage/meta/global --secret=$SYNC_MASTER_SECRET --as_header`. See [examples/put.bash](https://github.com/mozilla-services/syncstorage-rs/blob/master/tools/examples/put.bash) for an example of this.


Use `-h` for help.
