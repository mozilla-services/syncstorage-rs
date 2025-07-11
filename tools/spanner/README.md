# Spanner Tools and Scripts

These tools are supplemental scripts for working with the Google Cloud Platform. Follow [the general installation instructions](https://cloud.google.com/spanner/docs/getting-started/python/), as well as fetch the proper service account credentials file.

Remember, the `GOOGLE_APPLICATION_CREDENTIALS` environment variable should point to the absolute path location of your service account credential file.

e.g. 
```bash
GOOGLE_APPLICATION_CREDENTIALS=`pwd`/keys/project-id-service-cred.json venv/bin/python purge_ttl.py
```
See each script for details about function and use.

## Dependencies and Environment Setup:
To use the syncstorage-rs spanner untilities, you'll need a Python =>3.12 development environment with `Poetry` installed. These scripts typically run in a deployed GCP workflow, so their Dockerfile config will generally prepare all of this for you. If you're running them ad-hoc, you'll need to follow these instructions. You can also directly call the script using `Poetry` as described in step 5.

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
4. Install version, create virtualenv, activate and install dependencies from inside the `spanner/` directory.
**Note** you can simply install dependencies, not create a virtual environment and invoke the script using `poetry run`.

    ```shell
    $ cd syncstorage-rs/tools/spanner
    # pyenv version install
    $ pyenv install 3.10

    # creates named, associated virtualenv
    $ pyenv virtualenv 3.10 spanner # or whatever project name you like.
    $ pyenv local spanner # activates virtual env whenever you enter this directory. 

    # Install dependencies
    $ pip install poetry
    $ poetry install
    ```

5. In general, to run the script with the Poetry managed dependencies - once you're already in your virtual env - run the following (more details in #3):
Ex. `poetry run python purge_ttl.py`
