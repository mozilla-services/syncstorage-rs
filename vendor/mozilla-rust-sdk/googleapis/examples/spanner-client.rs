use std::error::Error;

use googleapis::spanner;

fn main() -> Result<(), Box<dyn Error>> {
    // An example database inside Mozilla's Spanner instance.
    let database = "projects/mozilla-rust-sdk-dev/instances/mozilla-spanner-dev/databases/mydb";

    // Create a Spanner client.
    let client = spanner::Client::new(database)?;

    Ok(())
}
