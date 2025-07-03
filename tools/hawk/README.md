# Make a Hawk compatible Auth header

## Dependencies and Environment Setup:
To use the syncstorage-rs `make_hawk_token.py` script, you'll need a Python =>3.10 development environment with `Poetry` installed. You can also directly call the script using `Poetry` as described in step 5.

The easiest solution recommended to use `pyenv` and the `pyenv-virtualenv` plugin for your virtual environments
as a way to isolate the dependencies from other directories.
1. Install `pyenv` using the [latest documentation](https://github.com/pyenv/pyenv#installation) for your platform.
2. Follow the instructions to install the `pyenv-virtualenv` plugin.
See the [pyenv-virtualenv](https://github.com/pyenv/pyenv-virtualenv) documentation.
3. Ensure you've added `pyenv` and `pyenv-virtualenv` to your PATH.

    Ex:
    ```shell
    export PATH="$HOME/.pyenv/bin:$PATH"
    eval "$(pyenv init -)"
    eval "$(pyenv virtualenv-init -)"
    ```
4. Install version, create virtualenv, activate and install dependencies from inside the `hawk/` directory.
**Note** you can simply install dependencies, not create a virtual environment and invoke the script using `poetry run`.

    ```shell
    $ cd syncstorage-rs/tools/hawk
    # pyenv version install
    $ pyenv install 3.10

    # creates named, associated virtualenv
    $ pyenv virtualenv 3.10 hawk # or whatever project name you like.
    $ pyenv local hawk # activates virtual env whenever you enter this directory. 

    # Install dependencies
    $ pip install poetry
    $ poetry install
    ```

5. In general, to run the script with the Poetry managed dependencies - once you're already in your virtual env - run the following (more details in #3):
`poetry run python make_hawk_token.py`

## Create a Token Header:

You'll need to pass along your `SYNC_MASTER_SECRET` and the uri you'll be testing in order to generate a valid Hawk Id:

`poetry run python make_hawk_token.py --uri /1.5/1/storage/meta/global --secret=$SYNC_MASTER_SECRET --as_header`

** For testing against uri's using methods other than GET, you'll need to pass along the `--method` flag to generate your token. Ie, `poetry run python make_hawk_token.py --method PUT --uri /1.5/1/storage/meta/global --secret=$SYNC_MASTER_SECRET --as_header`. See [examples/put.bash](https://github.com/mozilla-services/syncstorage-rs/blob/master/tools/examples/put.bash) for an example of this.

Use `-h` for help.

By default, with no passed arguments, a default Hawk Id token will be generated mapping to your localhost:8000 with
the path `http://localhost:8000/1.5/1/storage/col2/`.