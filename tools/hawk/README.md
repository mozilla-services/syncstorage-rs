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

`venv/bin/python make_hawk_token.py`

Use `-h` for help.
