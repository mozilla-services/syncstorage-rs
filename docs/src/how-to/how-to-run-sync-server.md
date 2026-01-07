<a id="howto_run_sync15"></a>

# Run your Own Sync-1.5 Server (legacy)

Note: this is for the legacy `syncserver`. This guide is left here for those still possibly
self-hosting and using this method.

Mozilla does not provide any pre-packaged release of the Firefox Sync server.

The easiest way to install a Sync Server is to checkout our repository
and run a build in-place. Once this is done, Sync can be run behind
any Web Server that supports the `WSGI` protocol.

## Important Notes

The sync service uses [Mozilla accounts](https://wiki.mozilla.org/Identity/FirefoxAccounts)
for user authentication, which is a separate service and is not covered by this guide.

> **Note:** By default, a server set up using this guide will defer authentication
> to the Mozilla-hosted accounts server at `https://accounts.firefox.com`.

You can safely use the Mozilla-hosted Mozilla accounts server in combination
with a self-hosted sync storage server. The authentication and encryption
protocols are designed so that the account server does not know the user's
plaintext password, and therefore cannot access their stored sync data.

Alternatively, you can also refer to [How To Run an FxA Server](how-to-run-fxa.md) to control all aspects of the
system. The process for doing so is currently very experimental and not well
documented.

## Prerequisites

The various parts are using **Python 2.7** and **Virtualenv**. Make sure your
system has them, or install them:

- Python 2.7 downloads: <http://python.org/download/releases/2.7.6>
- Virtualenv: <http://pypi.python.org/pypi/virtualenv>

To build and run the server, you will also need to have these packages
installed:

- `python-dev`
- `make`
- `git`
- c and c++ compiler

For example, under a fresh Ubuntu, you can run this command to meet all
requirements:

```bash
$ sudo apt-get install python-dev git-core python-virtualenv g++
```

## Building the server

Get the latest version at <https://github.com/mozilla-services/syncserver> and
run the **build** command:

```bash
$ git clone https://github.com/mozilla-services/syncserver
$ cd syncserver
$ make build
```

This command will create an isolated Python environment and pull all the
required dependencies in it. A **local/bin** directory is created and contains
a **gunicorn** command that can be used to run the server.

If you like, you can run the testsuite to make sure everything is working
properly:
```bash
$ make test
```

## Basic Configuration

The server is configured using an ini-like file to specify various runtime
settings. The file `syncserver.ini` will provide a useful starting point.

There is one setting that you *must* specify before running the server: the
client-visible URL for the service. Open `./syncserver.ini` and locate the
following lines:
```ini
[syncserver]
public_url = http://localhost:5000/
```

The default value of `public_url` will work for testing purposes on your local
machine. For final deployment, change it to the external, publicly-visible URL
of your server.

By default the server will use an in-memory database for storage, meaning that
any sync data will be lost on server restart. You will almost certainly want
to configure a more permanent database, which can be done with the `sqluri`
setting:
```ini
[syncserver]
sqluri = sqlite:////path/to/database/file.db
```

This setting will accept any [SQLAlchemy](http://www.sqlalchemy.org/)
database URI; for example the following would connect to a mysql server:
```ini
[syncserver]
sqluri = pymysql://username:password@db.example.com/sync
```

## Running the Server

Now you can run the server using gunicorn and the provided `syncserver.ini`
file. The simplest way is to use the Makefile like this:
```bash
$ make serve
```

Or if you'd like to pass additional arguments to gunicorn, like this:
```bash
$ local/bin/gunicorn --threads 4 --paste syncserver.ini
```

Once the server is launched, you need to tell Firefox about its location.

To configure desktop Firefox to talk to your new Sync server, go to
`about:config`, search for `identity.sync.tokenserver.uri` and change its value
to be the public URL of your server with a path of `token/1.0/sync/1.5`:

- `identity.sync.tokenserver.uri`: `http://localhost:5000/token/1.0/sync/1.5`

Alternatively, if you're running your own Mozilla accounts server, and running
Firefox 52 or later, see the documentation on how to refer to `howto_run_fxa`
for how to configure your client for both Sync and Mozilla accounts with a
single preference.

Firefox for Android ("Daylight", versions 79 and later) does support using a
non-Mozilla-hosted Sync server. **Before logging in**, go to App Menu > Settings
> About Firefox and click the logo 5 times. You should see a "debug menu enabled"
notification. Go back to the main menu and you will see two options for a custom
account server and a custom Sync server. Set the Sync server to the URL given
above and then log in.

To configure Android Firefox 44 up to 78 to talk to your new Sync server, just set
the `identity.sync.tokenserver.uri` exactly as above **before signing in to
Mozilla accounts and Sync on your Android device**.

> **Important:** *after* creating the Android account, changes to
> `identity.sync.tokenserver.uri` will be *ignored*.
>
> If you need to change the URI, delete the Android account using the
> **Settings > Sync > Disconnect...** menu item, update the pref, and sign in again.
>
> Non-default TokenServer URLs are displayed in the **Settings > Sync** panel in
> Firefox for Android, so you should be able to verify your URL there.

Prior to Firefox 44, a custom add-on was needed to configure Firefox for Android.
For Firefox 43 and earlier, see the blog post:
[How to connect Firefox for Android to self-hosted Mozilla account and Firefox Sync servers](http://www.ncalexander.net/blog/2014/07/05/how-to-connect-firefox-for-android-to-self-hosted-services/)

(Prior to Firefox 42, the TokenServer preference name for Firefox Desktop was
`services.sync.tokenServerURI`. While the old preference name will work in
Firefox 42 and later, the new preference is recommended as the old preference
name will be reset when the user signs out from Sync causing potential confusion.)

Since Firefox 18, Firefox for iOS has support for custom sync servers. The settings
can be made in the Advanced Sync Settings in the Mozilla account section, which are
visible if you are not signed in with a Mozilla account and have enabled the debug mode
(tap 5 times on the version number). In order to use the custom sync server with Firefox 28,
the token server's url must not contain the path `/1.0/sync/1.5`. It is also important to
configure a custom account content server (you may use the default `https://accounts.firefox.com`).

## Further Configuration

Once the server is running and Firefox is syncing successfully, there are
further configuration options you can tweak in the `syncserver.ini` file.

The `secret` setting is used by the server to generate cryptographically-signed
authentication tokens. It is blank by default, which means the server will
randomly generate a new secret at startup. For long-lived server installations
this should be set to a persistent value, generated from a good source of
randomness. An easy way to generate such a value on posix-style systems is to do:
```bash
    $ head -c 20 /dev/urandom | sha1sum
    db8a203aed5fe3e4594d4b75990acb76242efd35  -
```

Then copy-paste the value into the config file like so:
```ini
[syncserver]
...other settings...
secret = db8a203aed5fe3e4594d4b75990acb76242efd35
```

The `identity_provider` setting controls which server service can issue identity
assertions for access to the service. By default it will accept identity
assertions from the Mozilla-hosted account server at `https://accounts.firefox.com`.
If you are hosting your own instance of Mozilla accounts, you should change this
to your own domain:
```ini
[syncserver]
...other settings...
identity_provider = https://accounts.example.com
```

The `allow_new_users` setting controls whether the server will accept requests
from previously-unseen users. It is allowed by default, but once you have
configured Firefox and successfully synced with your user account, additional
users can be disabled by setting:
```ini
[syncserver]
...other settings...
allow_new_users = false
```

## Updating the server

You should periodically update your code to make sure you've got the latest fixes.
The following commands will update syncserver in place:
```bash
$ cd /path/to/syncserver
$ git stash       # to save any local changes to the config file
$ git pull        # to fetch latest updates from github
$ git stash pop   # to re-apply any local changes to the config file
$ make build      # to pull in any updated dependencies
```

## Running behind a Web Server

The built-in server should not be used in production, as it does not really
support a lot of load.

If you want to set up a production server, you can use different web servers
that are compatible with the WSGI protocol. For example:

- *Apache* combined with *mod_wsgi*
- *NGinx* with *Gunicorn* or *uWSGI*

> **Note:** Remember, you must set the **syncserver.public_url** option to the
> client-visible URL of your server.
>
> For example, if your server will be located at `http://example.com/ff-sync/`,
> the `public_url` should be set to this value in your config file:
>
>`[syncserver]`
>`public_url = http://example.com/ff-sync/`

### Apache + mod_wsgi

Here's an example of an Apache 2.2 setup that uses mod_wsgi:
```xml
<Directory /path/to/syncserver>
    Order deny,allow
    Allow from all
</Directory>

<VirtualHost *:80>
    ServerName example.com
    DocumentRoot /path/to/syncserver
    WSGIProcessGroup sync
    WSGIDaemonProcess sync user=sync group=sync processes=2 threads=25 python-path=/path/to/syncserver/local/lib/python2.7/site-packages/
    WSGIPassAuthorization On
    WSGIScriptAlias / /path/to/syncserver/syncserver.wsgi
    CustomLog /var/log/apache2/example.com-access.log combined
    ErrorLog  /var/log/apache2/example.com-error.log
</VirtualHost>
```

Here's the equivalent setup for Apache 2.4, which uses a different syntax for access control:
```xml
<Directory /path/to/syncserver>
    Require all granted
</Directory>

<VirtualHost *:80>
    ServerName example.com
    DocumentRoot /path/to/syncserver
    WSGIProcessGroup sync
    WSGIDaemonProcess sync user=sync group=sync processes=2 threads=25 python-path=/path/to/syncserver/local/lib/python2.7/site-packages/
    WSGIPassAuthorization On
    WSGIScriptAlias / /path/to/syncserver/syncserver.wsgi
    CustomLog /var/log/apache2/example.com-access.log combined
    ErrorLog  /var/log/apache2/example.com-error.log
</VirtualHost>
```

We provide a **syncserver.wsgi** file for your convenience in the repository.
Before running Apache, edit the file and check that it loads the right `.ini`
file with its full path.

Some users have reported issues with outbound TLS connections when running under
Apache. If your server is giving 503 errors and the Apache error log mentions
`SysCallError`, you may be able to correct the problem by installing the pyopenssl library:
```bash
$ local/bin/pip install pyopenssl
```

### Nginx + Gunicorn

Tested with debian stable/squeeze

1. First install gunicorn in the syncserver python environment:

```bash
$ cd /usr/src/syncserver
$ local/bin/pip install gunicorn
```

2. Then enable gunicorn in the `syncserver.ini` file:

```ini
[server:main]
use = egg:gunicorn
host = 127.0.0.1
port = 5000
workers = 2
timeout = 60
```

3. Finally edit your nginx vhost file:
```bash
server {
        listen  443 ssl;
        server_name sync.example.com;

        ssl_certificate /path/to/your.crt;
        ssl_certificate_key /path/to/your.key;

        location / {
                proxy_set_header Host $http_host;
                proxy_set_header X-Forwarded-Proto $scheme;
                proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
                proxy_set_header X-Real-IP $remote_addr;
                proxy_redirect off;
                proxy_read_timeout 120;
                proxy_connect_timeout 10;
                proxy_pass http://127.0.0.1:5000/;
                }
        }
```

5. After restarting your nginx and syncserver you should be able to use the
sync server behind your nginx installation

> **Note:** If you see errors about a mismatch between **public_url** and
> **application_url**, you may need to tell gunicorn that it should trust the
> **X-Forwarded-Proto** header being sent by nginx. Add the following to the
> gunicorn configuration in `syncserver.ini`:
>
>    `forwarded_allow_ips = *`

> **Note:** If you see errors about "client sent too long header line" in your
> nginx logs, you may need to configure nginx to allow large client header
> buffers by adding this to the nginx config:
>
>    `large_client_header_buffers 4 8k;`

### Nginx + uWSGI

1. Install uWSGI and its Python 2 plugin
2. Start it with the following options:
```bash
    uwsgi --plugins python27 --manage-script-name \
        --mount /<location>=/path/to/syncserver/syncserver.wsgi \
        --socket /path/to/uwsgi.sock
```

3. Use the following nginx configuration:
```bash
location /<location>/ {
    include uwsgi_params;
    uwsgi_pass unix:/path/to/uwsgi.sock;
}
```

## Things that still need to be Documented

- periodic pruning of expired sync data

## Asking for help

Don't hesitate to jump online and ask us for help:

- on Element (<https://chat.mozilla.org>) in the
  [#sync channel](https://chat.mozilla.org/#/room/#sync:mozilla.org)

- File an issue: [Syncstorage-rs GitHub Issues](https://github.com/mozilla-services/syncstorage-rs/issues).