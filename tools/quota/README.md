# Generate bookmarks to test Quota

## Installation
This presumes you have python3 installed. It is recommended you use a
virtual environment (see `python-virtualenv` or `python3 -m venv`). I prefer
to have the virtual environment under the parent `tools` directory so that it
may be easily shared between all of the tools, but that's up to you.

## Running
Once you've configured and activated your python environment, you can run
`python gen_bookmarks.py`

This will generate a file containing roughly 2GB of bookmarks. See
`python gen_bookmarks.py --help` for additional options.

These bookmarks may be imported into Firefox using the [standard HTML import mechanism](https://support.mozilla.org/en-US/kb/import-ie-favorites-other-computer).
