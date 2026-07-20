# A collection of Sync Tools and utilities

See each directory for details:

* [hawk](hawk) - a tool for generating test HAWK authorization headers
* [spanner](spanner) - Google Cloud Platform Spanner tools for maintenance and testing
* [payload-link-dataflow](payload-link-dataflow) - custom Dataflow flex template feeding the payload-link reconciliation pipeline
* [payload-reconciler](payload-reconciler) - Pub/Sub-driven reconciler that finalizes and garbage-collects offloaded GCS payload objects
## Installation

These tools are mostly written in python. It is recommended that you create a commonly shared virtual environment using something like:

`python3 -m venv venv`

to create a `/venv` directory. To activate this, call `sh /venv/bin/activate`.

Script dependencies can be installed via poetry:

```shell
pip install poetry
poetry install
```

for each tool.
