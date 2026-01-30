# Open API Documentation

## OpenAPI / Swagger UI

This project uses [utoipa](https://crates.io/crates/utoipa) and [utoipa-swagger-ui](https://crates.io/crates/utoipa-swagger-ui) to provide interactive API documentation.

### Accessing the Documentation

#### On GitHub Pages (Static Documentation):

The project automatically publishes API documentation to GitHub Pages:

- **Main Documentation**: https://mozilla-services.github.io/syncstorage-rs/
- **Rust API Docs (cargo doc)**: https://mozilla-services.github.io/syncstorage-rs/api/
- **OpenAPI/Swagger UI**: https://mozilla-services.github.io/syncstorage-rs/swagger-ui/

#### When the service is running (live deployment):
It is suggested to use the stage instance of Sync when playing with the API,
though you may also interact with your data in the production instance.

The Prod and Stage environments below will be available as a drop-down in the SwaggerUI:
- **Stage**: `https://sync-us-west1-g.sync.services.allizom.org`.
- **Prod**: `https://sync-1-us-west1-g.sync.services.mozilla.com`.

URLs for Swagger and OpenAPI Spec:
- **Swagger UI (Interactive)**: `https://<your-deployment-url>/swagger-ui/`
- **OpenAPI Spec (JSON)**: `https://<your-deployment-url>/api-doc/openapi.json`

Replace `<your-deployment-url>` with:
- **Production/Stage**: [Add your prod/stage URL here]
- **Local Development**: `http://localhost:8000` (or your configured port)

### API Endpoints

The API is organized into three main categories:

#### Syncstorage Endpoints
Endpoints for Firefox Sync data storage operations:
- `GET /1.5/{uid}/info/collections` - Get collection timestamps
- `GET /1.5/{uid}/info/collection_counts` - Get collection counts
- `GET /1.5/{uid}/info/collection_usage` - Get collection usage
- `GET /1.5/{uid}/info/configuration` - Get server configuration
- `GET /1.5/{uid}/info/quota` - Get quota information
- `DELETE /1.5/{uid}/storage` - Delete all user data
- `GET /1.5/{uid}/storage/{collection}` - Get BSOs from a collection
- `POST /1.5/{uid}/storage/{collection}` - Add or update BSOs
- `DELETE /1.5/{uid}/storage/{collection}` - Delete a collection or BSOs
- `GET /1.5/{uid}/storage/{collection}/{bso}` - Get a specific BSO
- `PUT /1.5/{uid}/storage/{collection}/{bso}` - Create or update a BSO
- `DELETE /1.5/{uid}/storage/{collection}/{bso}` - Delete a specific BSO

#### Tokenserver Endpoints
Endpoints for Sync node allocation and authentication:
- `GET /1.0/{application}/{version}` - Get sync token
- `GET /__heartbeat__` - Tokenserver health check

#### Dockerflow Endpoints
Service health and monitoring endpoints:
- `GET /__heartbeat__` - Service health check
- `GET /__lbheartbeat__` - Load balancer health check
- `GET /__version__` - Service version information

### Exploring the Sync API
To aid in exploring your own Sync API with Swagger, you may want to acquire your UID and other details about your Sync account. The easiest way to do so is to use the About Sync Extension. Note that this extension only works on Desktop. 

[Firefox Extensions Page for About Sync](https://addons.mozilla.org/en-US/firefox/addon/about-sync/)
[GitHub Repository for About Sync](https://github.com/mozilla-extensions/aboutsync)


### Maintenance

When adding new endpoints:
1. Add `#[utoipa::path(...)]` annotation to the handler function.
2. Add the handler path to `ApiDoc` in `syncserver/src/server/mod.rs`
3. If using custom types, derive `ToSchema` on request/response structs.
4. Run `cargo run --example generate_openapi_spec` to verify the spec generates correctly. Follow instructions below.

### Generating the OpenAPI Spec Locally
If you don't want to compile the Sync server on your machine to view the API docs, follow these instructions:

#### Use `make api-prev`
We created a handy Makefile command called `make api-prev` that automatically generates the specification file, runs Swagger in Docker and opens your browser to `localhost:8080`. See the steps below to understand this process. Note this attempts to be platform agnostic, but might require some adaptation depending on your operating system.

Commands to generate the OpenAPI specification without running the server:

```bash
# Generate the spec to stdout
cargo run --example generate_openapi_spec

# Save to a file
cargo run --example generate_openapi_spec > openapi.json
```

Other options: 
1. **Use Docker** (simplest - used in `make api-prev`):
This option requires you to have run `cargo run --example generate_openapi_spec > openapi.json`.

   ```bash
   docker run -p 8080:8080 -e SWAGGER_JSON=/openapi.json -v $(pwd)/openapi.json:/openapi.json swaggerapi/swagger-ui
   ```
   Then open http://localhost:8080

2. **Use online Swagger Editor**:
   - Go to https://editor.swagger.io/
   - Copy the contents of `openapi.json`
   - Paste into the editor
   - View the interactive documentation

3. **Use VS Code extension**:
   - Install "OpenAPI (Swagger) Editor" extension
   - Open `openapi.json` in VS Code
   - Click "Preview Swagger" to view interactive docs

### Publishing to GitHub Pages

The `.github/workflows/publish-docs.yaml` workflow automatically publishes these docs:

1. **Generates the OpenAPI spec** using the `generate_openapi_spec` example file.
2. **Downloads Swagger UI** from the official GitHub releases.
3. **Replaces the default example Swagger API** with your Sync API:
   - The default Swagger UI comes configured to display a demo "Pet Store" API
   - We use `sed` to replace `https://petstore.swagger.io/v2/swagger.json` with our `openapi.json`
4. **Deploys everything to GitHub Pages** at:
   - https://mozilla-services.github.io/syncstorage-rs/swagger-ui/

The workflow runs in parallel:
- `build-mdbook` job: Builds mdBook docs + Rust cargo docs
- `build-openapi` job: Generates OpenAPI spec + sets up Swagger UI
- `combine-and-prepare` job: Combines both outputs
- `deploy` job: Deploys to GitHub Pages
