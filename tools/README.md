# A collection of Sync Tools and utilities

See each directory for details:

* [hawk](hawk) - a tool for generating test HAWK authorization headers
* [spanner](spanner) - Google Cloud Platform Spanner tools for maintenance and testing
* [user_migration](user_migration) - scripts for dumping and moving user data from SQL to Spanner

## Installation

These tools are mostly written in python. It is recommended that you create a commonly shared virtual environment using something like:

`python3 -m venv venv`

to create a `/venv` directory. To activate this, call `sh /venv/bin/activate`.

Script dependencies can be installed via `pip install -r requirements.txt` for each tool.
