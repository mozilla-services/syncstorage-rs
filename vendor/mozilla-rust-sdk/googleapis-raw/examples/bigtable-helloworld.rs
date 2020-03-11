use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use futures::prelude::*;
use googleapis_raw::bigtable::admin::v2::{
    bigtable_instance_admin::GetClusterRequest,
    bigtable_instance_admin_grpc::BigtableInstanceAdminClient,
    bigtable_table_admin::CreateTableRequest, bigtable_table_admin::DeleteTableRequest,
    bigtable_table_admin::ListTablesRequest, bigtable_table_admin_grpc::BigtableTableAdminClient,
    instance::Cluster, table::ColumnFamily, table::GcRule, table::Table,
};
use googleapis_raw::bigtable::v2::{
    bigtable::MutateRowsRequest, bigtable::MutateRowsRequest_Entry, bigtable_grpc::BigtableClient,
    data::Mutation, data::Mutation_SetCell,
};
use googleapis_raw::empty::Empty;
use grpcio::{Channel, ChannelBuilder, ChannelCredentials, ClientUnaryReceiver, EnvBuilder};
use protobuf::well_known_types::Duration;
use protobuf::RepeatedField;

fn timestamp() -> u128 {
    let start = SystemTime::now();
    let time = start
        .duration_since(UNIX_EPOCH)
        .expect("Failed to fetch timestamp");
    time.as_micros()
}

/// Create a new channel used for the different types of clients
fn connect(endpoint: &str) -> Channel {
    // Set up the gRPC environment.
    let env = Arc::new(EnvBuilder::new().build());
    let creds =
        ChannelCredentials::google_default_credentials().expect("No Google credentials found");

    // Create a channel to connect to Gcloud.
    ChannelBuilder::new(env.clone())
        // Set the max size to correspond to server-side limits.
        .max_send_message_len(1 << 28)
        .max_receive_message_len(1 << 28)
        .secure_connect(&endpoint, creds)
}

/// Returns the cluster information
///
fn get_cluster(
    client: &BigtableInstanceAdminClient,
    cluster_id: &String,
) -> ::grpcio::Result<Cluster> {
    println!("Get cluster information");
    let mut request = GetClusterRequest::new();
    request.set_name(cluster_id.to_string());
    client.get_cluster(&request)
}

/// Lists all tables for a given cluster
///
fn list_tables(client: &BigtableTableAdminClient, instance_id: &String) {
    println!("List all existing tables");
    let mut request = ListTablesRequest::new();
    request.set_parent(instance_id.clone());
    match client.list_tables(&request) {
        Ok(response) => {
            response
                .get_tables()
                .iter()
                .for_each(|table| println!("  table: {:?}", table));
        }
        Err(error) => println!("Failed to list tables: {}", error),
    }
}

/// Create a new table in the BigTable cluster
///
fn create_table(
    client: &BigtableTableAdminClient,
    instance_id: &String,
    table_name: &String,
    table: Table,
) -> ::grpcio::Result<Table> {
    println!("Creating table {}", table_name);
    let mut request = CreateTableRequest::new();
    request.set_parent(instance_id.clone());
    request.set_table(table);
    request.set_table_id("hello-world".to_string());
    client.create_table(&request)
}

/// Deletes a table asynchronously, returns a future
fn delete_table_async(
    client: &BigtableTableAdminClient,
    table_name: &String,
) -> grpcio::Result<ClientUnaryReceiver<Empty>> {
    println!("Deleting the {} table", table_name);
    let mut request = DeleteTableRequest::new();
    request.set_name(table_name.clone());
    client.delete_table_async(&request)
}

fn main() -> Result<(), Box<dyn Error>> {
    // BigTable project id
    let project_id = String::from("mozilla-rust-sdk-dev");
    // The BigTable instance id
    let instance_id = String::from("projects/mozilla-rust-sdk-dev/instances/mozilla-rust-sdk");
    // The cluster id
    let cluster_id = String::from(
        "projects/mozilla-rust-sdk-dev/instances/mozilla-rust-sdk/clusters/mozilla-rust-sdk-c1",
    );
    // common table endpoint
    let endpoint = "bigtable.googleapis.com";
    // Google Cloud configuration.
    let admin_endpoint = "bigtableadmin.googleapis.com";
    // The table name
    let table_name =
        String::from("projects/mozilla-rust-sdk-dev/instances/mozilla-rust-sdk/tables/hello-world");

    let column_family_id = "cf1";

    // Create a Bigtable client.
    let channel = connect(admin_endpoint);
    let client = BigtableInstanceAdminClient::new(channel.clone());

    // display cluster information
    let cluster = get_cluster(&client, &cluster_id)?;
    dbg!(cluster);

    // create admin client for tables
    let admin_client = BigtableTableAdminClient::new(channel.clone());

    // display current tables
    list_tables(&admin_client, &instance_id);

    // create a new table with a custom column family / gc rule
    let mut duration = Duration::new();
    duration.set_seconds(60 * 60 * 24 * 5);
    let mut gc_rule = GcRule::new();
    gc_rule.set_max_num_versions(2);
    gc_rule.set_max_age(duration);
    let mut column_family = ColumnFamily::new();
    column_family.set_gc_rule(gc_rule);
    let mut hash_map = HashMap::new();
    hash_map.insert(column_family_id.to_string(), column_family);
    let mut table = Table::new();
    table.set_column_families(hash_map);
    match create_table(&admin_client, &instance_id, &table_name, table) {
        Ok(table) => println!("  table {:?} created", table),
        Err(error) => println!("  failed to created table: {}", error),
    }

    // insert entries into new table
    println!("Insert entries into table");

    let greetings = vec!["Hello World!", "Hello Cloud!", "Hello Rust!"];
    let mut mutation_requests = Vec::new();
    let column = "greeting";
    for (i, greeting) in greetings.iter().enumerate() {
        let row_key = format!("greeting{}", i);

        let mut set_cell = Mutation_SetCell::new();
        set_cell.set_column_qualifier(column.to_string().into_bytes());
        set_cell.set_timestamp_micros(-1);
        set_cell.set_value(greeting.to_string().into_bytes());
        set_cell.set_family_name(column_family_id.to_string());

        let mut mutation = Mutation::new();
        mutation.set_set_cell(set_cell);

        let mut request = MutateRowsRequest_Entry::new();
        request.set_row_key(row_key.into_bytes());
        request.set_mutations(RepeatedField::from_vec(vec![mutation]));

        mutation_requests.push(request);
    }

    let channel = connect(endpoint);
    let client = BigtableClient::new(channel.clone());
    let mut request = MutateRowsRequest::new();
    request.set_table_name(table_name.to_string());
    request.set_entries(RepeatedField::from_vec(mutation_requests));

    // apply changes and check responses
    let response = client
        .mutate_rows(&request)?
        .collect()
        .into_future()
        .wait()?;
    for response in response.iter() {
        for entry in response.get_entries().iter() {
            let status = entry.get_status();
            println!(
                "  entry index: {}, status: {} - {}",
                entry.get_index(),
                status.code,
                status.message
            );
        }
    }

    // display all tables, should include new table
    list_tables(&admin_client, &instance_id);

    // delete the table
    delete_table_async(&admin_client, &table_name)?.wait()?;

    // list of tables should not have deleted table
    list_tables(&admin_client, &instance_id);

    Ok(())
}
