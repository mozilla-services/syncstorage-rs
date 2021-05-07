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

use std::sync::Arc;

use futures::executor::block_on;
use googleapis_raw::spanner::v1::{
    spanner::{CreateSessionRequest, ExecuteSqlRequest},
    spanner_grpc::SpannerClient,
};
use grpcio::{CallOption, ChannelBuilder, ChannelCredentials, EnvBuilder, MetadataBuilder};

async fn async_main() {
    // An example database inside Mozilla's Spanner instance.
    let database = "projects/mozilla-rust-sdk-dev/instances/mozilla-spanner-dev/databases/mydb";

    // Google Cloud configuration.
    let endpoint = "spanner.googleapis.com:443";

    // Set up the gRPC environment.
    let env = Arc::new(EnvBuilder::new().build());
    let creds = ChannelCredentials::google_default_credentials().unwrap();

    // Create a Spanner client.
    let chan = ChannelBuilder::new(env.clone())
        .max_send_message_len(100 << 20)
        .max_receive_message_len(100 << 20)
        .secure_connect(&endpoint, creds);
    let client = SpannerClient::new(chan);

    // Connect to the instance and create a Spanner session.
    let mut req = CreateSessionRequest::new();
    req.database = database.to_string();
    let mut meta = MetadataBuilder::new();
    meta.add_str("google-cloud-resource-prefix", database)
        .unwrap();
    meta.add_str("x-goog-api-client", "googleapis-rs").unwrap();
    let opt = CallOption::default().headers(meta.build());
    let session = client.create_session_opt(&req, opt).unwrap();

    // Prepare a SQL command to execute.
    let mut req = ExecuteSqlRequest::new();
    req.session = session.get_name().to_string();
    req.sql = "select * from planets".to_string();

    // Execute the command synchronously.
    let out = client.execute_sql(&req).unwrap();
    dbg!(out);

    // Execute the command asynchronously.
    let fut = client.execute_sql_async(&req).unwrap();
    let out = fut.await.unwrap();
    dbg!(out);
}

fn main() {
    block_on(async_main());
}
