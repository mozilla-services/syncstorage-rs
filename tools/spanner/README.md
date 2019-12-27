# Spanner Tools and Scripts

These tools are supplimental scripts for working with the Google Cloud Platform. Follow [the general installation instructions](https://cloud.google.com/spanner/docs/getting-started/python/), as well as fetch the proper service account credentials file.

Remember, the `GOOGLE_APPLICATION_CREDENTIALS` environment variable should point to the absolute path location of your service account credential file.

e.g. 
```bash
GOOGLE_APPLICATION_CREDENTIALS=`pwd`/keys/project-id-service-cred.json venv/bin/python purge_ttl.py
```
See each script for details about function and use.
