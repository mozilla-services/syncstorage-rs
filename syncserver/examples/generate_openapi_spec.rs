// Simple binary to generate the OpenAPI spec without running Sync.
// Run with: cargo run --example generate_openapi_spec

use syncserver::server::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let openapi = ApiDoc::openapi();
    let json = serde_json::to_string_pretty(&openapi).expect("Failed to serialize OpenAPI spec");
    println!("{}", json);
}
