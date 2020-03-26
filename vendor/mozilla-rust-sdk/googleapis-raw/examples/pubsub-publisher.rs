use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::SystemTime;

use futures::prelude::*;
use googleapis_raw::pubsub::v1::{
    pubsub::AcknowledgeRequest, pubsub::ExpirationPolicy, pubsub::GetSubscriptionRequest,
    pubsub::GetTopicRequest, pubsub::PublishRequest, pubsub::PublishResponse,
    pubsub::PubsubMessage, pubsub::PullRequest, pubsub::PushConfig, pubsub::Subscription,
    pubsub::Topic, pubsub_grpc::PublisherClient, pubsub_grpc::SubscriberClient,
};
use grpcio::{Channel, ChannelBuilder, ChannelCredentials, ClientUnaryReceiver, EnvBuilder};
use protobuf::RepeatedField;

/// Creates a topic or finds an existing one, then returns the topic
///
fn find_or_create_topic(client: &PublisherClient, topic_name: &str) -> ::grpcio::Result<Topic> {
    // find topic
    println!("Finding topic {}", topic_name);
    let mut request = GetTopicRequest::new();
    request.set_topic(topic_name.to_string());
    if let Ok(topic) = client.get_topic(&request) {
        println!("Found topic: {}", topic.get_name());
        return Ok(topic);
    } else {
        println!("Topic not found");
    }

    // otherwise create topic
    println!("Creating topic {}", topic_name);
    let mut labels = HashMap::new();
    labels.insert("environment".to_string(), "test".to_string());
    let mut topic = Topic::new();
    topic.set_name(topic_name.to_string());
    topic.set_labels(labels);
    client.create_topic(&topic)
}

/// Creates a subscription or finds an existing one
///
fn find_or_create_subscription(
    client: &SubscriberClient,
    subscription_name: &str,
    topic_name: &str,
) -> ::grpcio::Result<Subscription> {
    // find subscription
    println!(
        "Finding subscription {} for topic {}",
        subscription_name, topic_name
    );
    let mut request = GetSubscriptionRequest::new();
    request.set_subscription(subscription_name.to_string());
    if let Ok(subscription) = client.get_subscription(&request) {
        println!("Found subscription: {}", subscription.get_name());
        return Ok(subscription);
    } else {
        println!("Subscription not found");
    }

    // create a new subscription
    println!("Creating a new subscription {}", subscription_name);
    let mut labels = HashMap::new();
    labels.insert("environment".to_string(), "test".to_string());
    let mut attributes = HashMap::new();
    attributes.insert("attribute".to_string(), "hello".to_string());
    let mut push_config = PushConfig::new();
    let mut expiration_policy = ExpirationPolicy::new();
    let mut expiration_duration = protobuf::well_known_types::Duration::new();
    let mut subscription = Subscription::new();
    push_config.set_attributes(attributes);
    expiration_duration.set_seconds(60 * 60 * 48);
    expiration_policy.set_ttl(expiration_duration.clone());
    subscription.set_name(subscription_name.to_string());
    subscription.set_topic(topic_name.to_string());
    subscription.set_ack_deadline_seconds(20);
    // subscription.set_expiration_policy(expiration_policy);
    // subscription.set_message_retention_duration(expiration_duration.clone());
    // subscription.set_push_config(push_config);
    // subscription.set_labels(labels);

    client.create_subscription(&subscription)
}

fn timestamp_in_seconds() -> u64 {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    timestamp.as_secs()
}

/// Creates a new PubSubMessage instance
///
fn create_pubsub_msg(message: &str) -> PubsubMessage {
    println!("Publishing message: {}", message);
    let mut timestamp = ::protobuf::well_known_types::Timestamp::new();
    timestamp.set_seconds(timestamp_in_seconds() as i64);

    let mut pubsub_msg = PubsubMessage::new();
    pubsub_msg.set_data(message.to_string().into_bytes());
    pubsub_msg.set_publish_time(timestamp);
    pubsub_msg
}

/// Publishes a message asynchronously, returning a future
///
fn publish_msg_async(
    client: &PublisherClient,
    topic: &Topic,
    messages: Vec<String>,
) -> ::grpcio::Result<ClientUnaryReceiver<PublishResponse>> {
    let pub_messages = messages.iter().map(|msg| create_pubsub_msg(msg)).collect();

    let mut request = PublishRequest::new();
    request.set_topic(topic.get_name().to_string());
    request.set_messages(RepeatedField::from_vec(pub_messages));
    client.publish_async(&request)
}

/// Create a new channel used for the different types of clients
///
fn connect(endpoint: &str) -> Channel {
    // Set up the gRPC environment.
    let env = Arc::new(EnvBuilder::new().build());
    let creds =
        ChannelCredentials::google_default_credentials().expect("No Google redentials found");

    // Create a channel to connect to Gcloud.
    ChannelBuilder::new(env.clone())
        // Set the max size to correspond to server-side limits.
        .max_send_message_len(1 << 28)
        .max_receive_message_len(1 << 28)
        .secure_connect(&endpoint, creds)
}

fn main() -> Result<(), Box<dyn Error>> {
    // API endpoint
    let endpoint = "pubsub.googleapis.com";
    // GCloud project id
    let project_id = "mozilla-rust-sdk-dev";

    // create client
    let channel = connect(&endpoint);
    let publisher = PublisherClient::new(channel.clone());

    // get topic
    let topic_name = format!("projects/{}/topics/greetings", project_id);
    let topic = dbg!(find_or_create_topic(&publisher, &topic_name)?);

    // publish a number of greeting messages
    let greetings = vec!["hello", "hi", "hola", "bonjour", "ahoi"];
    let messages = greetings.iter().map(|g| g.to_string()).collect();
    publish_msg_async(&publisher, &topic, messages)?.wait()?;

    // create a subscriber to consume these messages
    let subscription_name = format!("projects/{}/subscriptions/sub-greetings", project_id);
    let subscriber = SubscriberClient::new(channel.clone());

    // get subscription
    let subscription = find_or_create_subscription(&subscriber, &subscription_name, &topic_name)?;

    // Pubsub Subscription Pull, receive all messages
    println!("Pulling messages from subscription {:?}", subscription);
    let mut request = PullRequest::new();
    request.set_subscription(subscription_name.to_string());
    request.set_max_messages(10);

    loop {
        let future = subscriber.pull_async(&request)?;
        let response = future.wait()?;
        let pubsub_messages = response.get_received_messages();

        println!("Handling {} messages", pubsub_messages.len());
        for pubsub_message in pubsub_messages {
            println!("  >> message: {:?}", pubsub_message);
            let ack_id = pubsub_message.get_ack_id().to_string();

            let mut request = AcknowledgeRequest::new();
            request.set_subscription(subscription_name.to_string());
            request.set_ack_ids(RepeatedField::from_vec(vec![ack_id]));
            subscriber.acknowledge(&request)?;
        }

        // once all messages are handled leave
        if pubsub_messages.is_empty() {
            break;
        }
    }

    Ok(())
}
