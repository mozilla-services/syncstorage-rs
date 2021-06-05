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

//! Spanner client.

use std::sync::Arc;

use googleapis_raw::spanner::v1::{
    spanner::{CreateSessionRequest, Session},
    spanner_grpc::SpannerClient,
};
use grpcio::{CallOption, ChannelBuilder, ChannelCredentials, EnvBuilder, MetadataBuilder};

#[allow(dead_code)]
pub struct Client {
    database: String,
    client: SpannerClient,
    session: Session,
}

impl Client {
    /// Creates a new Spanner client.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use googleapis::spanner;
    ///
    /// let db = "projects/my_project/instances/my_instance/databases/my_database";
    /// let client = spanner::Client::new(db);
    /// ```
    pub fn new(database: &str) -> crate::Result<Client> {
        let database = database.to_string();
        let endpoint = "spanner.googleapis.com:443";

        // Set up the gRPC environment.
        let env = Arc::new(EnvBuilder::new().build());
        let creds = ChannelCredentials::google_default_credentials()?;

        // Create a Spanner client.
        let chan = ChannelBuilder::new(Arc::clone(&env))
            .max_send_message_len(100 << 20)
            .max_receive_message_len(100 << 20)
            .secure_connect(&endpoint, creds);
        let client = SpannerClient::new(chan);

        let mut req = CreateSessionRequest::new();
        req.set_database(database.to_string());
        let mut meta = MetadataBuilder::new();
        meta.add_str("google-cloud-resource-prefix", &database)?;
        meta.add_str("x-goog-api-client", "googleapis-rs")?;
        let opt = CallOption::default().headers(meta.build());
        let session = client.create_session_opt(&req, opt)?;

        Ok(Client {
            database,
            client,
            session,
        })
    }
}
