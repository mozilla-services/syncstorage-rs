// Simple binary to generate the administrative OpenAPI spec without running Sync.
// Run with: cargo run --example generate_admin_openapi_spec

use syncserver::server::AdminApiDoc;
use utoipa::OpenApi;

fn main() {
    let openapi = AdminApiDoc::openapi();
    let json = serde_json::to_string_pretty(&openapi).expect("Failed to serialize OpenAPI spec");
    println!("{}", json);
}
