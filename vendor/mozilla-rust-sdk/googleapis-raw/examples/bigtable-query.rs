use std::error::Error;
use std::sync::Arc;

use futures::prelude::*;
use googleapis_raw::bigtable::v2::{bigtable::ReadRowsRequest, bigtable_grpc::BigtableClient};
use grpcio::{ChannelBuilder, ChannelCredentials, EnvBuilder};

fn main() -> Result<(), Box<dyn Error>> {
    // An example database inside Mozilla's Bigtable instance.
    let table = "projects/mozilla-rust-sdk-dev/instances/mozilla-rust-sdk/tables/prezzy";

    // Google Cloud configuration.
    let endpoint = "bigtable.googleapis.com:443";

    // Set up the gRPC environment.
    let env = Arc::new(EnvBuilder::new().build());
    let creds = ChannelCredentials::google_default_credentials()?;

    // Create a Bigtable client.
    let chan = ChannelBuilder::new(env.clone())
        // Set the max size to correspond to server-side limits.
        .max_send_message_len(1 << 28)
        .max_receive_message_len(1 << 28)
        .secure_connect(&endpoint, creds);
    let client = BigtableClient::new(chan);

    // Create a request to read all rows.
    let mut req = ReadRowsRequest::new();
    req.table_name = table.to_string();

    // Iterate over the rows and print them.
    let mut stream = client.read_rows(&req)?;
    while let (Some(row), s) = stream.into_future().wait().map_err(|(e, _)| e)? {
        stream = s;
        dbg!(row);
    }

    Ok(())
}
