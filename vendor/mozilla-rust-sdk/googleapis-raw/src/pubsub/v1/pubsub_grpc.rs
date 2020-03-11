// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

#![cfg_attr(rustfmt, rustfmt_skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]

const METHOD_PUBLISHER_CREATE_TOPIC: ::grpcio::Method<super::pubsub::Topic, super::pubsub::Topic> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Publisher/CreateTopic",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_PUBLISHER_UPDATE_TOPIC: ::grpcio::Method<super::pubsub::UpdateTopicRequest, super::pubsub::Topic> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Publisher/UpdateTopic",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_PUBLISHER_PUBLISH: ::grpcio::Method<super::pubsub::PublishRequest, super::pubsub::PublishResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Publisher/Publish",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_PUBLISHER_GET_TOPIC: ::grpcio::Method<super::pubsub::GetTopicRequest, super::pubsub::Topic> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Publisher/GetTopic",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_PUBLISHER_LIST_TOPICS: ::grpcio::Method<super::pubsub::ListTopicsRequest, super::pubsub::ListTopicsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Publisher/ListTopics",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_PUBLISHER_LIST_TOPIC_SUBSCRIPTIONS: ::grpcio::Method<super::pubsub::ListTopicSubscriptionsRequest, super::pubsub::ListTopicSubscriptionsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Publisher/ListTopicSubscriptions",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_PUBLISHER_LIST_TOPIC_SNAPSHOTS: ::grpcio::Method<super::pubsub::ListTopicSnapshotsRequest, super::pubsub::ListTopicSnapshotsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Publisher/ListTopicSnapshots",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_PUBLISHER_DELETE_TOPIC: ::grpcio::Method<super::pubsub::DeleteTopicRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Publisher/DeleteTopic",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct PublisherClient {
    client: ::grpcio::Client,
}

impl PublisherClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        PublisherClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn create_topic_opt(&self, req: &super::pubsub::Topic, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::Topic> {
        self.client.unary_call(&METHOD_PUBLISHER_CREATE_TOPIC, req, opt)
    }

    pub fn create_topic(&self, req: &super::pubsub::Topic) -> ::grpcio::Result<super::pubsub::Topic> {
        self.create_topic_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_topic_async_opt(&self, req: &super::pubsub::Topic, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Topic>> {
        self.client.unary_call_async(&METHOD_PUBLISHER_CREATE_TOPIC, req, opt)
    }

    pub fn create_topic_async(&self, req: &super::pubsub::Topic) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Topic>> {
        self.create_topic_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_topic_opt(&self, req: &super::pubsub::UpdateTopicRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::Topic> {
        self.client.unary_call(&METHOD_PUBLISHER_UPDATE_TOPIC, req, opt)
    }

    pub fn update_topic(&self, req: &super::pubsub::UpdateTopicRequest) -> ::grpcio::Result<super::pubsub::Topic> {
        self.update_topic_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_topic_async_opt(&self, req: &super::pubsub::UpdateTopicRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Topic>> {
        self.client.unary_call_async(&METHOD_PUBLISHER_UPDATE_TOPIC, req, opt)
    }

    pub fn update_topic_async(&self, req: &super::pubsub::UpdateTopicRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Topic>> {
        self.update_topic_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn publish_opt(&self, req: &super::pubsub::PublishRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::PublishResponse> {
        self.client.unary_call(&METHOD_PUBLISHER_PUBLISH, req, opt)
    }

    pub fn publish(&self, req: &super::pubsub::PublishRequest) -> ::grpcio::Result<super::pubsub::PublishResponse> {
        self.publish_opt(req, ::grpcio::CallOption::default())
    }

    pub fn publish_async_opt(&self, req: &super::pubsub::PublishRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::PublishResponse>> {
        self.client.unary_call_async(&METHOD_PUBLISHER_PUBLISH, req, opt)
    }

    pub fn publish_async(&self, req: &super::pubsub::PublishRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::PublishResponse>> {
        self.publish_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_topic_opt(&self, req: &super::pubsub::GetTopicRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::Topic> {
        self.client.unary_call(&METHOD_PUBLISHER_GET_TOPIC, req, opt)
    }

    pub fn get_topic(&self, req: &super::pubsub::GetTopicRequest) -> ::grpcio::Result<super::pubsub::Topic> {
        self.get_topic_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_topic_async_opt(&self, req: &super::pubsub::GetTopicRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Topic>> {
        self.client.unary_call_async(&METHOD_PUBLISHER_GET_TOPIC, req, opt)
    }

    pub fn get_topic_async(&self, req: &super::pubsub::GetTopicRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Topic>> {
        self.get_topic_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_topics_opt(&self, req: &super::pubsub::ListTopicsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::ListTopicsResponse> {
        self.client.unary_call(&METHOD_PUBLISHER_LIST_TOPICS, req, opt)
    }

    pub fn list_topics(&self, req: &super::pubsub::ListTopicsRequest) -> ::grpcio::Result<super::pubsub::ListTopicsResponse> {
        self.list_topics_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_topics_async_opt(&self, req: &super::pubsub::ListTopicsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::ListTopicsResponse>> {
        self.client.unary_call_async(&METHOD_PUBLISHER_LIST_TOPICS, req, opt)
    }

    pub fn list_topics_async(&self, req: &super::pubsub::ListTopicsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::ListTopicsResponse>> {
        self.list_topics_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_topic_subscriptions_opt(&self, req: &super::pubsub::ListTopicSubscriptionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::ListTopicSubscriptionsResponse> {
        self.client.unary_call(&METHOD_PUBLISHER_LIST_TOPIC_SUBSCRIPTIONS, req, opt)
    }

    pub fn list_topic_subscriptions(&self, req: &super::pubsub::ListTopicSubscriptionsRequest) -> ::grpcio::Result<super::pubsub::ListTopicSubscriptionsResponse> {
        self.list_topic_subscriptions_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_topic_subscriptions_async_opt(&self, req: &super::pubsub::ListTopicSubscriptionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::ListTopicSubscriptionsResponse>> {
        self.client.unary_call_async(&METHOD_PUBLISHER_LIST_TOPIC_SUBSCRIPTIONS, req, opt)
    }

    pub fn list_topic_subscriptions_async(&self, req: &super::pubsub::ListTopicSubscriptionsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::ListTopicSubscriptionsResponse>> {
        self.list_topic_subscriptions_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_topic_snapshots_opt(&self, req: &super::pubsub::ListTopicSnapshotsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::ListTopicSnapshotsResponse> {
        self.client.unary_call(&METHOD_PUBLISHER_LIST_TOPIC_SNAPSHOTS, req, opt)
    }

    pub fn list_topic_snapshots(&self, req: &super::pubsub::ListTopicSnapshotsRequest) -> ::grpcio::Result<super::pubsub::ListTopicSnapshotsResponse> {
        self.list_topic_snapshots_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_topic_snapshots_async_opt(&self, req: &super::pubsub::ListTopicSnapshotsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::ListTopicSnapshotsResponse>> {
        self.client.unary_call_async(&METHOD_PUBLISHER_LIST_TOPIC_SNAPSHOTS, req, opt)
    }

    pub fn list_topic_snapshots_async(&self, req: &super::pubsub::ListTopicSnapshotsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::ListTopicSnapshotsResponse>> {
        self.list_topic_snapshots_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_topic_opt(&self, req: &super::pubsub::DeleteTopicRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_PUBLISHER_DELETE_TOPIC, req, opt)
    }

    pub fn delete_topic(&self, req: &super::pubsub::DeleteTopicRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_topic_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_topic_async_opt(&self, req: &super::pubsub::DeleteTopicRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_PUBLISHER_DELETE_TOPIC, req, opt)
    }

    pub fn delete_topic_async(&self, req: &super::pubsub::DeleteTopicRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_topic_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait Publisher {
    fn create_topic(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::Topic, sink: ::grpcio::UnarySink<super::pubsub::Topic>);
    fn update_topic(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::UpdateTopicRequest, sink: ::grpcio::UnarySink<super::pubsub::Topic>);
    fn publish(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::PublishRequest, sink: ::grpcio::UnarySink<super::pubsub::PublishResponse>);
    fn get_topic(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::GetTopicRequest, sink: ::grpcio::UnarySink<super::pubsub::Topic>);
    fn list_topics(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::ListTopicsRequest, sink: ::grpcio::UnarySink<super::pubsub::ListTopicsResponse>);
    fn list_topic_subscriptions(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::ListTopicSubscriptionsRequest, sink: ::grpcio::UnarySink<super::pubsub::ListTopicSubscriptionsResponse>);
    fn list_topic_snapshots(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::ListTopicSnapshotsRequest, sink: ::grpcio::UnarySink<super::pubsub::ListTopicSnapshotsResponse>);
    fn delete_topic(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::DeleteTopicRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
}

pub fn create_publisher<S: Publisher + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_PUBLISHER_CREATE_TOPIC, move |ctx, req, resp| {
        instance.create_topic(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_PUBLISHER_UPDATE_TOPIC, move |ctx, req, resp| {
        instance.update_topic(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_PUBLISHER_PUBLISH, move |ctx, req, resp| {
        instance.publish(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_PUBLISHER_GET_TOPIC, move |ctx, req, resp| {
        instance.get_topic(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_PUBLISHER_LIST_TOPICS, move |ctx, req, resp| {
        instance.list_topics(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_PUBLISHER_LIST_TOPIC_SUBSCRIPTIONS, move |ctx, req, resp| {
        instance.list_topic_subscriptions(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_PUBLISHER_LIST_TOPIC_SNAPSHOTS, move |ctx, req, resp| {
        instance.list_topic_snapshots(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_PUBLISHER_DELETE_TOPIC, move |ctx, req, resp| {
        instance.delete_topic(ctx, req, resp)
    });
    builder.build()
}

const METHOD_SUBSCRIBER_CREATE_SUBSCRIPTION: ::grpcio::Method<super::pubsub::Subscription, super::pubsub::Subscription> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/CreateSubscription",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_GET_SUBSCRIPTION: ::grpcio::Method<super::pubsub::GetSubscriptionRequest, super::pubsub::Subscription> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/GetSubscription",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_UPDATE_SUBSCRIPTION: ::grpcio::Method<super::pubsub::UpdateSubscriptionRequest, super::pubsub::Subscription> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/UpdateSubscription",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_LIST_SUBSCRIPTIONS: ::grpcio::Method<super::pubsub::ListSubscriptionsRequest, super::pubsub::ListSubscriptionsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/ListSubscriptions",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_DELETE_SUBSCRIPTION: ::grpcio::Method<super::pubsub::DeleteSubscriptionRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/DeleteSubscription",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_MODIFY_ACK_DEADLINE: ::grpcio::Method<super::pubsub::ModifyAckDeadlineRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/ModifyAckDeadline",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_ACKNOWLEDGE: ::grpcio::Method<super::pubsub::AcknowledgeRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/Acknowledge",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_PULL: ::grpcio::Method<super::pubsub::PullRequest, super::pubsub::PullResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/Pull",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_STREAMING_PULL: ::grpcio::Method<super::pubsub::StreamingPullRequest, super::pubsub::StreamingPullResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Duplex,
    name: "/google.pubsub.v1.Subscriber/StreamingPull",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_MODIFY_PUSH_CONFIG: ::grpcio::Method<super::pubsub::ModifyPushConfigRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/ModifyPushConfig",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_GET_SNAPSHOT: ::grpcio::Method<super::pubsub::GetSnapshotRequest, super::pubsub::Snapshot> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/GetSnapshot",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_LIST_SNAPSHOTS: ::grpcio::Method<super::pubsub::ListSnapshotsRequest, super::pubsub::ListSnapshotsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/ListSnapshots",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_CREATE_SNAPSHOT: ::grpcio::Method<super::pubsub::CreateSnapshotRequest, super::pubsub::Snapshot> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/CreateSnapshot",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_UPDATE_SNAPSHOT: ::grpcio::Method<super::pubsub::UpdateSnapshotRequest, super::pubsub::Snapshot> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/UpdateSnapshot",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_DELETE_SNAPSHOT: ::grpcio::Method<super::pubsub::DeleteSnapshotRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/DeleteSnapshot",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SUBSCRIBER_SEEK: ::grpcio::Method<super::pubsub::SeekRequest, super::pubsub::SeekResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.Subscriber/Seek",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct SubscriberClient {
    client: ::grpcio::Client,
}

impl SubscriberClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        SubscriberClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn create_subscription_opt(&self, req: &super::pubsub::Subscription, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::Subscription> {
        self.client.unary_call(&METHOD_SUBSCRIBER_CREATE_SUBSCRIPTION, req, opt)
    }

    pub fn create_subscription(&self, req: &super::pubsub::Subscription) -> ::grpcio::Result<super::pubsub::Subscription> {
        self.create_subscription_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_subscription_async_opt(&self, req: &super::pubsub::Subscription, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Subscription>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_CREATE_SUBSCRIPTION, req, opt)
    }

    pub fn create_subscription_async(&self, req: &super::pubsub::Subscription) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Subscription>> {
        self.create_subscription_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_subscription_opt(&self, req: &super::pubsub::GetSubscriptionRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::Subscription> {
        self.client.unary_call(&METHOD_SUBSCRIBER_GET_SUBSCRIPTION, req, opt)
    }

    pub fn get_subscription(&self, req: &super::pubsub::GetSubscriptionRequest) -> ::grpcio::Result<super::pubsub::Subscription> {
        self.get_subscription_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_subscription_async_opt(&self, req: &super::pubsub::GetSubscriptionRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Subscription>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_GET_SUBSCRIPTION, req, opt)
    }

    pub fn get_subscription_async(&self, req: &super::pubsub::GetSubscriptionRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Subscription>> {
        self.get_subscription_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_subscription_opt(&self, req: &super::pubsub::UpdateSubscriptionRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::Subscription> {
        self.client.unary_call(&METHOD_SUBSCRIBER_UPDATE_SUBSCRIPTION, req, opt)
    }

    pub fn update_subscription(&self, req: &super::pubsub::UpdateSubscriptionRequest) -> ::grpcio::Result<super::pubsub::Subscription> {
        self.update_subscription_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_subscription_async_opt(&self, req: &super::pubsub::UpdateSubscriptionRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Subscription>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_UPDATE_SUBSCRIPTION, req, opt)
    }

    pub fn update_subscription_async(&self, req: &super::pubsub::UpdateSubscriptionRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Subscription>> {
        self.update_subscription_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_subscriptions_opt(&self, req: &super::pubsub::ListSubscriptionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::ListSubscriptionsResponse> {
        self.client.unary_call(&METHOD_SUBSCRIBER_LIST_SUBSCRIPTIONS, req, opt)
    }

    pub fn list_subscriptions(&self, req: &super::pubsub::ListSubscriptionsRequest) -> ::grpcio::Result<super::pubsub::ListSubscriptionsResponse> {
        self.list_subscriptions_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_subscriptions_async_opt(&self, req: &super::pubsub::ListSubscriptionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::ListSubscriptionsResponse>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_LIST_SUBSCRIPTIONS, req, opt)
    }

    pub fn list_subscriptions_async(&self, req: &super::pubsub::ListSubscriptionsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::ListSubscriptionsResponse>> {
        self.list_subscriptions_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_subscription_opt(&self, req: &super::pubsub::DeleteSubscriptionRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_SUBSCRIBER_DELETE_SUBSCRIPTION, req, opt)
    }

    pub fn delete_subscription(&self, req: &super::pubsub::DeleteSubscriptionRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_subscription_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_subscription_async_opt(&self, req: &super::pubsub::DeleteSubscriptionRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_DELETE_SUBSCRIPTION, req, opt)
    }

    pub fn delete_subscription_async(&self, req: &super::pubsub::DeleteSubscriptionRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_subscription_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn modify_ack_deadline_opt(&self, req: &super::pubsub::ModifyAckDeadlineRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_SUBSCRIBER_MODIFY_ACK_DEADLINE, req, opt)
    }

    pub fn modify_ack_deadline(&self, req: &super::pubsub::ModifyAckDeadlineRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.modify_ack_deadline_opt(req, ::grpcio::CallOption::default())
    }

    pub fn modify_ack_deadline_async_opt(&self, req: &super::pubsub::ModifyAckDeadlineRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_MODIFY_ACK_DEADLINE, req, opt)
    }

    pub fn modify_ack_deadline_async(&self, req: &super::pubsub::ModifyAckDeadlineRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.modify_ack_deadline_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn acknowledge_opt(&self, req: &super::pubsub::AcknowledgeRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_SUBSCRIBER_ACKNOWLEDGE, req, opt)
    }

    pub fn acknowledge(&self, req: &super::pubsub::AcknowledgeRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.acknowledge_opt(req, ::grpcio::CallOption::default())
    }

    pub fn acknowledge_async_opt(&self, req: &super::pubsub::AcknowledgeRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_ACKNOWLEDGE, req, opt)
    }

    pub fn acknowledge_async(&self, req: &super::pubsub::AcknowledgeRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.acknowledge_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn pull_opt(&self, req: &super::pubsub::PullRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::PullResponse> {
        self.client.unary_call(&METHOD_SUBSCRIBER_PULL, req, opt)
    }

    pub fn pull(&self, req: &super::pubsub::PullRequest) -> ::grpcio::Result<super::pubsub::PullResponse> {
        self.pull_opt(req, ::grpcio::CallOption::default())
    }

    pub fn pull_async_opt(&self, req: &super::pubsub::PullRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::PullResponse>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_PULL, req, opt)
    }

    pub fn pull_async(&self, req: &super::pubsub::PullRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::PullResponse>> {
        self.pull_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn streaming_pull_opt(&self, opt: ::grpcio::CallOption) -> ::grpcio::Result<(::grpcio::ClientDuplexSender<super::pubsub::StreamingPullRequest>, ::grpcio::ClientDuplexReceiver<super::pubsub::StreamingPullResponse>)> {
        self.client.duplex_streaming(&METHOD_SUBSCRIBER_STREAMING_PULL, opt)
    }

    pub fn streaming_pull(&self) -> ::grpcio::Result<(::grpcio::ClientDuplexSender<super::pubsub::StreamingPullRequest>, ::grpcio::ClientDuplexReceiver<super::pubsub::StreamingPullResponse>)> {
        self.streaming_pull_opt(::grpcio::CallOption::default())
    }

    pub fn modify_push_config_opt(&self, req: &super::pubsub::ModifyPushConfigRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_SUBSCRIBER_MODIFY_PUSH_CONFIG, req, opt)
    }

    pub fn modify_push_config(&self, req: &super::pubsub::ModifyPushConfigRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.modify_push_config_opt(req, ::grpcio::CallOption::default())
    }

    pub fn modify_push_config_async_opt(&self, req: &super::pubsub::ModifyPushConfigRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_MODIFY_PUSH_CONFIG, req, opt)
    }

    pub fn modify_push_config_async(&self, req: &super::pubsub::ModifyPushConfigRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.modify_push_config_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_snapshot_opt(&self, req: &super::pubsub::GetSnapshotRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::Snapshot> {
        self.client.unary_call(&METHOD_SUBSCRIBER_GET_SNAPSHOT, req, opt)
    }

    pub fn get_snapshot(&self, req: &super::pubsub::GetSnapshotRequest) -> ::grpcio::Result<super::pubsub::Snapshot> {
        self.get_snapshot_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_snapshot_async_opt(&self, req: &super::pubsub::GetSnapshotRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Snapshot>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_GET_SNAPSHOT, req, opt)
    }

    pub fn get_snapshot_async(&self, req: &super::pubsub::GetSnapshotRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Snapshot>> {
        self.get_snapshot_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_snapshots_opt(&self, req: &super::pubsub::ListSnapshotsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::ListSnapshotsResponse> {
        self.client.unary_call(&METHOD_SUBSCRIBER_LIST_SNAPSHOTS, req, opt)
    }

    pub fn list_snapshots(&self, req: &super::pubsub::ListSnapshotsRequest) -> ::grpcio::Result<super::pubsub::ListSnapshotsResponse> {
        self.list_snapshots_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_snapshots_async_opt(&self, req: &super::pubsub::ListSnapshotsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::ListSnapshotsResponse>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_LIST_SNAPSHOTS, req, opt)
    }

    pub fn list_snapshots_async(&self, req: &super::pubsub::ListSnapshotsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::ListSnapshotsResponse>> {
        self.list_snapshots_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_snapshot_opt(&self, req: &super::pubsub::CreateSnapshotRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::Snapshot> {
        self.client.unary_call(&METHOD_SUBSCRIBER_CREATE_SNAPSHOT, req, opt)
    }

    pub fn create_snapshot(&self, req: &super::pubsub::CreateSnapshotRequest) -> ::grpcio::Result<super::pubsub::Snapshot> {
        self.create_snapshot_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_snapshot_async_opt(&self, req: &super::pubsub::CreateSnapshotRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Snapshot>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_CREATE_SNAPSHOT, req, opt)
    }

    pub fn create_snapshot_async(&self, req: &super::pubsub::CreateSnapshotRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Snapshot>> {
        self.create_snapshot_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_snapshot_opt(&self, req: &super::pubsub::UpdateSnapshotRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::Snapshot> {
        self.client.unary_call(&METHOD_SUBSCRIBER_UPDATE_SNAPSHOT, req, opt)
    }

    pub fn update_snapshot(&self, req: &super::pubsub::UpdateSnapshotRequest) -> ::grpcio::Result<super::pubsub::Snapshot> {
        self.update_snapshot_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_snapshot_async_opt(&self, req: &super::pubsub::UpdateSnapshotRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Snapshot>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_UPDATE_SNAPSHOT, req, opt)
    }

    pub fn update_snapshot_async(&self, req: &super::pubsub::UpdateSnapshotRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::Snapshot>> {
        self.update_snapshot_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_snapshot_opt(&self, req: &super::pubsub::DeleteSnapshotRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_SUBSCRIBER_DELETE_SNAPSHOT, req, opt)
    }

    pub fn delete_snapshot(&self, req: &super::pubsub::DeleteSnapshotRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_snapshot_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_snapshot_async_opt(&self, req: &super::pubsub::DeleteSnapshotRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_DELETE_SNAPSHOT, req, opt)
    }

    pub fn delete_snapshot_async(&self, req: &super::pubsub::DeleteSnapshotRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_snapshot_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn seek_opt(&self, req: &super::pubsub::SeekRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::pubsub::SeekResponse> {
        self.client.unary_call(&METHOD_SUBSCRIBER_SEEK, req, opt)
    }

    pub fn seek(&self, req: &super::pubsub::SeekRequest) -> ::grpcio::Result<super::pubsub::SeekResponse> {
        self.seek_opt(req, ::grpcio::CallOption::default())
    }

    pub fn seek_async_opt(&self, req: &super::pubsub::SeekRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::SeekResponse>> {
        self.client.unary_call_async(&METHOD_SUBSCRIBER_SEEK, req, opt)
    }

    pub fn seek_async(&self, req: &super::pubsub::SeekRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::pubsub::SeekResponse>> {
        self.seek_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait Subscriber {
    fn create_subscription(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::Subscription, sink: ::grpcio::UnarySink<super::pubsub::Subscription>);
    fn get_subscription(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::GetSubscriptionRequest, sink: ::grpcio::UnarySink<super::pubsub::Subscription>);
    fn update_subscription(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::UpdateSubscriptionRequest, sink: ::grpcio::UnarySink<super::pubsub::Subscription>);
    fn list_subscriptions(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::ListSubscriptionsRequest, sink: ::grpcio::UnarySink<super::pubsub::ListSubscriptionsResponse>);
    fn delete_subscription(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::DeleteSubscriptionRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn modify_ack_deadline(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::ModifyAckDeadlineRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn acknowledge(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::AcknowledgeRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn pull(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::PullRequest, sink: ::grpcio::UnarySink<super::pubsub::PullResponse>);
    fn streaming_pull(&mut self, ctx: ::grpcio::RpcContext, stream: ::grpcio::RequestStream<super::pubsub::StreamingPullRequest>, sink: ::grpcio::DuplexSink<super::pubsub::StreamingPullResponse>);
    fn modify_push_config(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::ModifyPushConfigRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn get_snapshot(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::GetSnapshotRequest, sink: ::grpcio::UnarySink<super::pubsub::Snapshot>);
    fn list_snapshots(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::ListSnapshotsRequest, sink: ::grpcio::UnarySink<super::pubsub::ListSnapshotsResponse>);
    fn create_snapshot(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::CreateSnapshotRequest, sink: ::grpcio::UnarySink<super::pubsub::Snapshot>);
    fn update_snapshot(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::UpdateSnapshotRequest, sink: ::grpcio::UnarySink<super::pubsub::Snapshot>);
    fn delete_snapshot(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::DeleteSnapshotRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn seek(&mut self, ctx: ::grpcio::RpcContext, req: super::pubsub::SeekRequest, sink: ::grpcio::UnarySink<super::pubsub::SeekResponse>);
}

pub fn create_subscriber<S: Subscriber + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_CREATE_SUBSCRIPTION, move |ctx, req, resp| {
        instance.create_subscription(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_GET_SUBSCRIPTION, move |ctx, req, resp| {
        instance.get_subscription(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_UPDATE_SUBSCRIPTION, move |ctx, req, resp| {
        instance.update_subscription(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_LIST_SUBSCRIPTIONS, move |ctx, req, resp| {
        instance.list_subscriptions(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_DELETE_SUBSCRIPTION, move |ctx, req, resp| {
        instance.delete_subscription(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_MODIFY_ACK_DEADLINE, move |ctx, req, resp| {
        instance.modify_ack_deadline(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_ACKNOWLEDGE, move |ctx, req, resp| {
        instance.acknowledge(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_PULL, move |ctx, req, resp| {
        instance.pull(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_duplex_streaming_handler(&METHOD_SUBSCRIBER_STREAMING_PULL, move |ctx, req, resp| {
        instance.streaming_pull(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_MODIFY_PUSH_CONFIG, move |ctx, req, resp| {
        instance.modify_push_config(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_GET_SNAPSHOT, move |ctx, req, resp| {
        instance.get_snapshot(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_LIST_SNAPSHOTS, move |ctx, req, resp| {
        instance.list_snapshots(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_CREATE_SNAPSHOT, move |ctx, req, resp| {
        instance.create_snapshot(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_UPDATE_SNAPSHOT, move |ctx, req, resp| {
        instance.update_snapshot(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_DELETE_SNAPSHOT, move |ctx, req, resp| {
        instance.delete_snapshot(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SUBSCRIBER_SEEK, move |ctx, req, resp| {
        instance.seek(ctx, req, resp)
    });
    builder.build()
}
