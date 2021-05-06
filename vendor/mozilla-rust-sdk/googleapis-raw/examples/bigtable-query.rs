// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::error::Error;
use std::sync::Arc;

use futures::prelude::*;
use futures::executor::block_on;
use googleapis_raw::bigtable::v2::{bigtable::ReadRowsRequest, bigtable_grpc::BigtableClient};
use grpcio::{ChannelBuilder, ChannelCredentials, EnvBuilder};

async fn async_main() {
    // An example database inside Mozilla's Bigtable instance.
    let table = "projects/mozilla-rust-sdk-dev/instances/mozilla-rust-sdk/tables/prezzy";

    // Google Cloud configuration.
    let endpoint = "bigtable.googleapis.com:443";

    // Set up the gRPC environment.
    let env = Arc::new(EnvBuilder::new().build());
    let creds = ChannelCredentials::google_default_credentials().unwrap();

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
    let mut stream = match client.read_rows(&req) {
        Ok(s) => s,
        Err(e) => {
            println!("Error: {:?}", e);
            return;
        },
    };
    while let (Some(row), s) = stream.into_future().await {
        stream = s;
        dbg!(row);
    }

}

fn main() {
    block_on(async_main())
}