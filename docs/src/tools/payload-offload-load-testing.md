# Load Testing Payload Offload

This provides a guide on running load tests (`tools/syncstorage-loadtest/`) on the
GCS payload offload path with Molotov in GCP. This focuses on using gcloud cli
commands and short scripts as opposed to the UI.

Two methods:

- **Method A, against dev env.** Real Spanner, real GCS, real change stream and
  reconciler. Payloads capped at whatever dev's limits allow. Use this to test
  the whole pipeline end to end. Note: it requires webservices-infra updates
  should you need to change application limits.
- **Method B, local syncserver on the VM.** Spanner emulator, real GCS, your own
  limits. No change stream or reconciler. Use this to test expanded payloads
  larger than the server's default ceiling. This allows for easier ad-hoc
  changing of application limits.

Both use the load tester's **direct access** mode, which creates its own Hawk
token from the master secret and bypasses Tokenserver.

---

## Prerequisites

### Check your gcloud configuration first

If you have a configuration for the Spanner emulator and it is enabled,
it has the bad habit of breaking everything in confusing ways. Check:

```console
gcloud config list
```

A configuration containing `auth/disable_credentials = true` or
`api_endpoint_overrides/spanner` sends every request out unauthenticated and
redirects Spanner calls to localhost. Switch to a normal configuration:

```console
gcloud config configurations activate default
gcloud auth login
```

Switch back with `gcloud config configurations activate <emulator-config>` when
you return to local emulator work.

### Access you need

| For | You need |
| --- | --- |
| Creating the VM | Compute admin on some project. `sync/developers` has **no** compute roles on the sync tenant projects, so use your own project under the developers folder. |
| Method A | `roles/secretmanager.secretAccessor` on the tenant project, to read the master secret. |
| Method B | Write access to a GCS bucket you control. |

Sync's tenant is not onboarded to PAM, so there is no just-in-time elevation
available. `gcloud pam entitlements search` returns nothing.

---

## Part 1: Create the load generator VM

Use your own project under the org's `developers` folder (get the folder_no in GCP)
rather than a tenant project.

```console
PROJ=<your-project-id>

gcloud projects create $PROJ --folder=<folder_no>
gcloud billing accounts list
gcloud billing projects link $PROJ --billing-account=<ACCOUNT_ID>
gcloud services enable compute.googleapis.com --project=$PROJ
```

New projects have no default VPC, because the org enforces
`compute.skipDefaultNetworkCreation`. Create one:

```console
gcloud compute networks create default --subnet-mode=auto --project=$PROJ
```

Firewall for IAP SSH required. Without this, the SSH in the next step fails
when no firewall rules are set. Org rules forbid opening tcp/22 to `0.0.0.0/0`:

```console
gcloud compute firewall-rules create allow-iap-ssh \
  --project=$PROJ --network=default \
  --direction=INGRESS --action=allow --rules=tcp:22 \
  --source-ranges=35.235.240.0/20
```

Create the instance in **us-west1**, matching where dev runs.
Always check region and match to limit latency.

```console
gcloud compute instances create sync-loadtest \
  --project=$PROJ \
  --zone=us-west1-b \
  --machine-type=n2-standard-8 \
  --image-family=ubuntu-2404-lts-amd64 --image-project=ubuntu-os-cloud \
  --boot-disk-size=200GB
```

> **Note on Machine family.** New projects get zero quota for newer families, so C4D
> fails with `Quota 'CPUS_PER_VM_FAMILY' exceeded. Limit: 0.0`. N2 and E2 draw
> from the general `CPUS` pool, which does have a default allocation. Check with
> `gcloud compute regions describe us-west1 --project=$PROJ --flatten="quotas[]"
> --format="table(quotas.metric,quotas.limit,quotas.usage)"`.

To Connect via terminal to instance (you'll likely use this command a lot):

```console
gcloud compute ssh sync-loadtest --project=$PROJ --zone=us-west1-b \
  --tunnel-through-iap
```

Install sync & load tests on the VM:

```console
sudo apt update && sudo apt install -y git python3 python3-venv
mkdir -p ~/src && cd ~/src
git clone https://github.com/mozilla-services/syncstorage-rs.git
cd syncstorage-rs/tools/syncstorage-loadtest
python3 -m venv venv && source venv/bin/activate
pip install poetry && poetry install
```

---

## Part 2: Method A - load test against dev

### Fetch the master secret programmatically

Do not copy and paste the secret. It can contain spaces and a paste
may result in confusing 401 errors that are just a mismatched token.

```console
gcloud auth login --no-launch-browser   # VM's own service account cannot read it

SECRET=$(gcloud secrets versions access latest --secret=dev-gke-app-secrets \
  --project=moz-fx-sync-nonprod \
  | python3 -c 'import sys,json; print(json.load(sys.stdin)["SYNC_MASTER_SECRET"])')

export SERVER_URL="https://dev.sync.nonprod.webservices.mozgcp.net#${SECRET}"
echo "${SERVER_URL%%#*}"
```

That last line prints the origin without revealing the secret. If empty,
stop: an unset `SERVER_URL` results in a fall back to
`https://token.stage.mozaws.net` and take the OAuth path instead.

### Verify authentication before running

```console
poetry run python -c "
import asyncio, aiohttp
from storage.client import StorageClient
async def m():
    async with aiohttp.ClientSession() as s:
        c = StorageClient(s)
        r, _ = await c.get('/info/collections', (200, 404))
        print(r.status)
asyncio.run(m())
"
```

Use `/info/collections`, not `/info/configuration`. The latter takes no auth
extractor at all and returns 200 to anyone, so it cannot tell you whether your
secret is right.

### Run

```console
LARGE_PAYLOAD_PROB=1.0 OFFLOAD_COLLECTIONS=bookmarks \
  poetry run molotov --max-runs 5 -cxv loadtest.py
```

`OFFLOAD_COLLECTIONS` must name a collection the server actually offloads.
Check `SYNC_SYNCSTORAGE__GCS_PAYLOAD_OFFLOAD_COLLECTIONS` in the deployment's
values file.

Then scale up, redirecting output to a file:

```console
LARGE_PAYLOAD_PROB=1.0 OFFLOAD_COLLECTIONS=bookmarks \
  nohup poetry run molotov --processes 1 --workers 5 --duration 600 -v loadtest.py \
  > /tmp/lt.log 2>&1 &
```

Ramp gradually. Dev is small, and a sudden jump to high concurrency will hit
its ceiling rather quickly.

### Verify offload actually occurred

```console
gcloud storage ls -r "gs://sync-nonprod-dev-syncstorage-payloads/**" \
  --project=moz-fx-sync-nonprod | grep -v '[/:]$' | tail -5
```

Then check at least one object was finalized by the reconciler:

```console
gcloud storage objects describe <OBJECT_URL> --project=moz-fx-sync-nonprod
```

Look for `custom_fields.committed: 'true'` and `custom_time` pinned to the far
future. The gap between `creation_time` and `update_time` is your
end-to-end reconciliation latency, which can be worth noting.

> Note: the metadata key is `custom_fields` in `gcloud storage` output, not
> `metadata`.

---

## Part 3: Method B, local syncserver with raised limits

Use this when you need payloads larger than the deployed server allows. There
is no runtime or client-side override: limits are read from the server's
environment at startup, which you can easily change on the fly.
It runs on whatever `/info/configuration`
is set to. Exceeding said server-side limits returns 413.

### Your own bucket

Do not use a shared environment's bucket. Make one you own:

```console
gcloud storage buckets create gs://<your-bucket> --project=$PROJ --location=us-west1
```

Add a lifecycle rule. These objects are never finalized in this setup, so
nothing else will clean them up. A 20 minute run can write hundreds of GB, so we
don't necessarily want these sticking around:

```console
echo '{"rule":[{"action":{"type":"Delete"},"condition":{"age":1}}]}' > /tmp/lc.json
gcloud storage buckets update gs://<your-bucket> --lifecycle-file=/tmp/lc.json --project=$PROJ
```

### Give the VM write access

The default compute service account is scoped to `devstorage.read_only`, so it
cannot write, regardless of IAM settings. Note that scopes can only change while the instance
is stopped:

```console
SA=$(gcloud compute instances describe sync-loadtest --project=$PROJ \
  --zone=us-west1-b --format="value(serviceAccounts[0].email)")

gcloud storage buckets add-iam-policy-binding gs://<your-bucket> \
  --member=serviceAccount:$SA --role=roles/storage.objectAdmin --project=$PROJ

gcloud compute instances stop sync-loadtest --project=$PROJ --zone=us-west1-b
gcloud compute instances set-service-account sync-loadtest \
  --project=$PROJ --zone=us-west1-b --service-account=$SA \
  --scopes=https://www.googleapis.com/auth/cloud-platform
gcloud compute instances start sync-loadtest --project=$PROJ --zone=us-west1-b
```

Disk contents survive the stop. Verify:

```console
echo hello | gcloud storage cp - gs://<your-bucket>/write-check.txt --project=$PROJ
gcloud storage rm gs://<your-bucket>/write-check.txt --project=$PROJ
```

Using the instance service account this way means no expiring credentials and
nothing to mount into the container.

### Build the Docker image

```console
sudo apt install -y docker.io docker-compose-v2 docker-buildx
sudo usermod -aG docker $USER && newgrp docker

cd ~/src/syncstorage-rs
DOCKER_BUILDKIT=1 nohup docker build -t app:build . > /tmp/build.log 2>&1 &
```

> `DOCKER_BUILDKIT=1` is required. The Dockerfile uses BuildKit cache mounts and
> the legacy builder fails with "the --mount option requires BuildKit".

Pulling the published image instead is much faster if you have Artifact
Registry read, after `gcloud auth configure-docker us-docker.pkg.dev`. Tag it as
`app:build` either way, because the compose file hardcodes that tag for its
setup container.

### Compose override

```console
cd ~/src/syncstorage-rs
cat > docker/docker-compose.offload-local.yaml <<'YAML'
services:
  syncserver:
    environment:
      SYNC_SYNCSTORAGE__GCS_PAYLOAD_BUCKET: <your-bucket>
      SYNC_SYNCSTORAGE__GCS_PAYLOAD_OFFLOAD_COLLECTIONS: bookmarks
      SYNC_SYNCSTORAGE__LIMITS__MAX_RECORD_PAYLOAD_BYTES: "20971520"
      SYNC_SYNCSTORAGE__LIMITS__MAX_POST_BYTES: "26214400"
      SYNC_SYNCSTORAGE__LIMITS__MAX_REQUEST_BYTES: "26218496"
YAML
```

No nginx runs in this stack, therefore the chart's `nginxClientMaxBodySize` (default
`2568k`) does not apply. It does apply to any deployed environment, though.
Note that  raising syncserver limits without it gets you a 413 from nginx,
before syncserver sees a request.

### Docker compose

```console
docker tag app:build syncstorage-rs:latest

docker compose -f docker/docker-compose.spanner.yaml \
               -f docker/docker-compose.offload-local.yaml up -d

docker compose -f docker/docker-compose.spanner.yaml ps
```

The compose default image is `syncstorage-rs:latest`, hence the tag. Otherwise
set `SYNCSTORAGE_RS_IMAGE=app:build` on every compose.

Confirm the limits are valid:

```console
curl -s http://localhost:8000/1.5/1/info/configuration | python3 -m json.tool
```

`max_record_payload_bytes` must show your raised value.

### Run

The master secret in this compose config is `secret0`:

```console
cd ~/src/syncstorage-rs/tools/syncstorage-loadtest
source venv/bin/activate
export SERVER_URL="http://localhost:8000#secret0"

LARGE_PAYLOAD_PROB=1.0 LARGE_PAYLOAD_SIZE=20971520 OFFLOAD_COLLECTIONS=bookmarks \
  poetry run molotov --max-runs 5 -cxv loadtest.py
```

Confirm the object sizes:

```console
gcloud storage ls -r --long "gs://<your-bucket>/**" --project=$PROJ | tail -6
```

### Sampling memory during a run

Memory can be easily measured here, and is a valuable performance metric.
Use the below command to sample it to a file (rather than watching a streaming display):

```console
nohup bash -c 'while true; do date -u +%H:%M:%S | tr "\n" " "; docker stats --no-stream --format "{{.MemUsage}} {{.CPUPerc}}" docker-syncserver-1; sleep 5; done' > /tmp/mem.log 2>&1 &
```

Then run the load tests. Then `pkill -f "docker stats"` and read `/tmp/mem.log`.

---

## Caveats

**Molotov hides its errors.** A failing `setup_session` is counted and the
traceback is discarded, so any auth or setup problem shows as
`SUCCESSES: 0 | FAILURES: 1` with nothing else.

Also note `_retry` only checks the HTTP status when the caller passes an
expected-statuses tuple. The scenario's `/info/configuration` call passes none,
so a 503 appears as a JSON decode error rather than "Response 503".

**Molotov's display can mangle terminals.** To save yourself the possible headache, always
redirect to a file for anything but a short test, and if your terminal
goes blank or stops echoing, run `reset`.

**Always redirect, never pipe.** Piping molotov's output through `head` or
`grep` produces basically unhelpful output.

**Co-locating the server and the load tester skews results.** Note that to some degree, Method B
results in both compete for the same CPU and memory. Use low concurrency, or alternatively a second VM.

**The emulator is not Spanner.** Method B's write throughput and latency numbers
aren't super helpful and indicative of live spanner performance.

**Cloud Armor Adaptive Protection is armed on nonprod.** The policy has an
`evaluateAdaptiveProtectionAutoDeploy()` rule that denies with 403.
403s partway through a sustained run against a deployed environment
is likely not the load tester.

**Check where the Spanner metrics live.** Spanner for the sync tenant is in a
separate GCPv1 project, and `sync/developers` has no access to it at all, so the
console is unavailable. Grafana's `gcp-v1-nonprod` datasource reads it with its
own credentials.

---

## Cleanup

```console
gcloud compute instances delete sync-loadtest --project=$PROJ --zone=us-west1-b
gcloud storage rm -r gs://<your-bucket>/** --project=$PROJ
```

Delete the instance when you are done. An idle load generator bills
indefinitely!
