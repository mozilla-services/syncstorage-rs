// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

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

const METHOD_STORAGE_DELETE_BUCKET_ACCESS_CONTROL: ::grpcio::Method<super::storage::DeleteBucketAccessControlRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/DeleteBucketAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_GET_BUCKET_ACCESS_CONTROL: ::grpcio::Method<super::storage::GetBucketAccessControlRequest, super::storage_resources::BucketAccessControl> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/GetBucketAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_INSERT_BUCKET_ACCESS_CONTROL: ::grpcio::Method<super::storage::InsertBucketAccessControlRequest, super::storage_resources::BucketAccessControl> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/InsertBucketAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_LIST_BUCKET_ACCESS_CONTROLS: ::grpcio::Method<super::storage::ListBucketAccessControlsRequest, super::storage_resources::ListBucketAccessControlsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/ListBucketAccessControls",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_UPDATE_BUCKET_ACCESS_CONTROL: ::grpcio::Method<super::storage::UpdateBucketAccessControlRequest, super::storage_resources::BucketAccessControl> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/UpdateBucketAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_PATCH_BUCKET_ACCESS_CONTROL: ::grpcio::Method<super::storage::PatchBucketAccessControlRequest, super::storage_resources::BucketAccessControl> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/PatchBucketAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_DELETE_BUCKET: ::grpcio::Method<super::storage::DeleteBucketRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/DeleteBucket",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_GET_BUCKET: ::grpcio::Method<super::storage::GetBucketRequest, super::storage_resources::Bucket> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/GetBucket",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_INSERT_BUCKET: ::grpcio::Method<super::storage::InsertBucketRequest, super::storage_resources::Bucket> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/InsertBucket",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_LIST_CHANNELS: ::grpcio::Method<super::storage::ListChannelsRequest, super::storage_resources::ListChannelsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/ListChannels",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_LIST_BUCKETS: ::grpcio::Method<super::storage::ListBucketsRequest, super::storage_resources::ListBucketsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/ListBuckets",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_LOCK_BUCKET_RETENTION_POLICY: ::grpcio::Method<super::storage::LockRetentionPolicyRequest, super::storage_resources::Bucket> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/LockBucketRetentionPolicy",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_GET_BUCKET_IAM_POLICY: ::grpcio::Method<super::storage::GetIamPolicyRequest, super::policy::Policy> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/GetBucketIamPolicy",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_SET_BUCKET_IAM_POLICY: ::grpcio::Method<super::storage::SetIamPolicyRequest, super::policy::Policy> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/SetBucketIamPolicy",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_TEST_BUCKET_IAM_PERMISSIONS: ::grpcio::Method<super::storage::TestIamPermissionsRequest, super::iam_policy::TestIamPermissionsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/TestBucketIamPermissions",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_PATCH_BUCKET: ::grpcio::Method<super::storage::PatchBucketRequest, super::storage_resources::Bucket> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/PatchBucket",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_UPDATE_BUCKET: ::grpcio::Method<super::storage::UpdateBucketRequest, super::storage_resources::Bucket> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/UpdateBucket",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_STOP_CHANNEL: ::grpcio::Method<super::storage::StopChannelRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/StopChannel",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_DELETE_DEFAULT_OBJECT_ACCESS_CONTROL: ::grpcio::Method<super::storage::DeleteDefaultObjectAccessControlRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/DeleteDefaultObjectAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_GET_DEFAULT_OBJECT_ACCESS_CONTROL: ::grpcio::Method<super::storage::GetDefaultObjectAccessControlRequest, super::storage_resources::ObjectAccessControl> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/GetDefaultObjectAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_INSERT_DEFAULT_OBJECT_ACCESS_CONTROL: ::grpcio::Method<super::storage::InsertDefaultObjectAccessControlRequest, super::storage_resources::ObjectAccessControl> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/InsertDefaultObjectAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_LIST_DEFAULT_OBJECT_ACCESS_CONTROLS: ::grpcio::Method<super::storage::ListDefaultObjectAccessControlsRequest, super::storage_resources::ListObjectAccessControlsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/ListDefaultObjectAccessControls",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_PATCH_DEFAULT_OBJECT_ACCESS_CONTROL: ::grpcio::Method<super::storage::PatchDefaultObjectAccessControlRequest, super::storage_resources::ObjectAccessControl> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/PatchDefaultObjectAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_UPDATE_DEFAULT_OBJECT_ACCESS_CONTROL: ::grpcio::Method<super::storage::UpdateDefaultObjectAccessControlRequest, super::storage_resources::ObjectAccessControl> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/UpdateDefaultObjectAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_DELETE_NOTIFICATION: ::grpcio::Method<super::storage::DeleteNotificationRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/DeleteNotification",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_GET_NOTIFICATION: ::grpcio::Method<super::storage::GetNotificationRequest, super::storage_resources::Notification> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/GetNotification",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_INSERT_NOTIFICATION: ::grpcio::Method<super::storage::InsertNotificationRequest, super::storage_resources::Notification> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/InsertNotification",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_LIST_NOTIFICATIONS: ::grpcio::Method<super::storage::ListNotificationsRequest, super::storage_resources::ListNotificationsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/ListNotifications",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_DELETE_OBJECT_ACCESS_CONTROL: ::grpcio::Method<super::storage::DeleteObjectAccessControlRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/DeleteObjectAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_GET_OBJECT_ACCESS_CONTROL: ::grpcio::Method<super::storage::GetObjectAccessControlRequest, super::storage_resources::ObjectAccessControl> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/GetObjectAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_INSERT_OBJECT_ACCESS_CONTROL: ::grpcio::Method<super::storage::InsertObjectAccessControlRequest, super::storage_resources::ObjectAccessControl> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/InsertObjectAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_LIST_OBJECT_ACCESS_CONTROLS: ::grpcio::Method<super::storage::ListObjectAccessControlsRequest, super::storage_resources::ListObjectAccessControlsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/ListObjectAccessControls",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_PATCH_OBJECT_ACCESS_CONTROL: ::grpcio::Method<super::storage::PatchObjectAccessControlRequest, super::storage_resources::ObjectAccessControl> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/PatchObjectAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_UPDATE_OBJECT_ACCESS_CONTROL: ::grpcio::Method<super::storage::UpdateObjectAccessControlRequest, super::storage_resources::ObjectAccessControl> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/UpdateObjectAccessControl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_COMPOSE_OBJECT: ::grpcio::Method<super::storage::ComposeObjectRequest, super::storage_resources::Object> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/ComposeObject",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_COPY_OBJECT: ::grpcio::Method<super::storage::CopyObjectRequest, super::storage_resources::Object> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/CopyObject",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_DELETE_OBJECT: ::grpcio::Method<super::storage::DeleteObjectRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/DeleteObject",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_GET_OBJECT: ::grpcio::Method<super::storage::GetObjectRequest, super::storage_resources::Object> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/GetObject",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_GET_OBJECT_MEDIA: ::grpcio::Method<super::storage::GetObjectMediaRequest, super::storage::GetObjectMediaResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::ServerStreaming,
    name: "/google.storage.v1.Storage/GetObjectMedia",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_INSERT_OBJECT: ::grpcio::Method<super::storage::InsertObjectRequest, super::storage_resources::Object> = ::grpcio::Method {
    ty: ::grpcio::MethodType::ClientStreaming,
    name: "/google.storage.v1.Storage/InsertObject",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_LIST_OBJECTS: ::grpcio::Method<super::storage::ListObjectsRequest, super::storage_resources::ListObjectsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/ListObjects",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_REWRITE_OBJECT: ::grpcio::Method<super::storage::RewriteObjectRequest, super::storage::RewriteResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/RewriteObject",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_START_RESUMABLE_WRITE: ::grpcio::Method<super::storage::StartResumableWriteRequest, super::storage::StartResumableWriteResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/StartResumableWrite",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_QUERY_WRITE_STATUS: ::grpcio::Method<super::storage::QueryWriteStatusRequest, super::storage::QueryWriteStatusResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/QueryWriteStatus",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_PATCH_OBJECT: ::grpcio::Method<super::storage::PatchObjectRequest, super::storage_resources::Object> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/PatchObject",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_UPDATE_OBJECT: ::grpcio::Method<super::storage::UpdateObjectRequest, super::storage_resources::Object> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/UpdateObject",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_GET_OBJECT_IAM_POLICY: ::grpcio::Method<super::storage::GetIamPolicyRequest, super::policy::Policy> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/GetObjectIamPolicy",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_SET_OBJECT_IAM_POLICY: ::grpcio::Method<super::storage::SetIamPolicyRequest, super::policy::Policy> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/SetObjectIamPolicy",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_TEST_OBJECT_IAM_PERMISSIONS: ::grpcio::Method<super::storage::TestIamPermissionsRequest, super::iam_policy::TestIamPermissionsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/TestObjectIamPermissions",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_WATCH_ALL_OBJECTS: ::grpcio::Method<super::storage::WatchAllObjectsRequest, super::storage_resources::Channel> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/WatchAllObjects",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_GET_SERVICE_ACCOUNT: ::grpcio::Method<super::storage::GetProjectServiceAccountRequest, super::storage_resources::ServiceAccount> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/GetServiceAccount",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_CREATE_HMAC_KEY: ::grpcio::Method<super::storage::CreateHmacKeyRequest, super::storage::CreateHmacKeyResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/CreateHmacKey",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_DELETE_HMAC_KEY: ::grpcio::Method<super::storage::DeleteHmacKeyRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/DeleteHmacKey",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_GET_HMAC_KEY: ::grpcio::Method<super::storage::GetHmacKeyRequest, super::storage_resources::HmacKeyMetadata> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/GetHmacKey",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_LIST_HMAC_KEYS: ::grpcio::Method<super::storage::ListHmacKeysRequest, super::storage::ListHmacKeysResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/ListHmacKeys",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_STORAGE_UPDATE_HMAC_KEY: ::grpcio::Method<super::storage::UpdateHmacKeyRequest, super::storage_resources::HmacKeyMetadata> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.storage.v1.Storage/UpdateHmacKey",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct StorageClient {
    client: ::grpcio::Client,
}

impl StorageClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        StorageClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn delete_bucket_access_control_opt(&self, req: &super::storage::DeleteBucketAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_STORAGE_DELETE_BUCKET_ACCESS_CONTROL, req, opt)
    }

    pub fn delete_bucket_access_control(&self, req: &super::storage::DeleteBucketAccessControlRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_bucket_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_bucket_access_control_async_opt(&self, req: &super::storage::DeleteBucketAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_STORAGE_DELETE_BUCKET_ACCESS_CONTROL, req, opt)
    }

    pub fn delete_bucket_access_control_async(&self, req: &super::storage::DeleteBucketAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_bucket_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_bucket_access_control_opt(&self, req: &super::storage::GetBucketAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::BucketAccessControl> {
        self.client.unary_call(&METHOD_STORAGE_GET_BUCKET_ACCESS_CONTROL, req, opt)
    }

    pub fn get_bucket_access_control(&self, req: &super::storage::GetBucketAccessControlRequest) -> ::grpcio::Result<super::storage_resources::BucketAccessControl> {
        self.get_bucket_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_bucket_access_control_async_opt(&self, req: &super::storage::GetBucketAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::BucketAccessControl>> {
        self.client.unary_call_async(&METHOD_STORAGE_GET_BUCKET_ACCESS_CONTROL, req, opt)
    }

    pub fn get_bucket_access_control_async(&self, req: &super::storage::GetBucketAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::BucketAccessControl>> {
        self.get_bucket_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn insert_bucket_access_control_opt(&self, req: &super::storage::InsertBucketAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::BucketAccessControl> {
        self.client.unary_call(&METHOD_STORAGE_INSERT_BUCKET_ACCESS_CONTROL, req, opt)
    }

    pub fn insert_bucket_access_control(&self, req: &super::storage::InsertBucketAccessControlRequest) -> ::grpcio::Result<super::storage_resources::BucketAccessControl> {
        self.insert_bucket_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn insert_bucket_access_control_async_opt(&self, req: &super::storage::InsertBucketAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::BucketAccessControl>> {
        self.client.unary_call_async(&METHOD_STORAGE_INSERT_BUCKET_ACCESS_CONTROL, req, opt)
    }

    pub fn insert_bucket_access_control_async(&self, req: &super::storage::InsertBucketAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::BucketAccessControl>> {
        self.insert_bucket_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_bucket_access_controls_opt(&self, req: &super::storage::ListBucketAccessControlsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ListBucketAccessControlsResponse> {
        self.client.unary_call(&METHOD_STORAGE_LIST_BUCKET_ACCESS_CONTROLS, req, opt)
    }

    pub fn list_bucket_access_controls(&self, req: &super::storage::ListBucketAccessControlsRequest) -> ::grpcio::Result<super::storage_resources::ListBucketAccessControlsResponse> {
        self.list_bucket_access_controls_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_bucket_access_controls_async_opt(&self, req: &super::storage::ListBucketAccessControlsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ListBucketAccessControlsResponse>> {
        self.client.unary_call_async(&METHOD_STORAGE_LIST_BUCKET_ACCESS_CONTROLS, req, opt)
    }

    pub fn list_bucket_access_controls_async(&self, req: &super::storage::ListBucketAccessControlsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ListBucketAccessControlsResponse>> {
        self.list_bucket_access_controls_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_bucket_access_control_opt(&self, req: &super::storage::UpdateBucketAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::BucketAccessControl> {
        self.client.unary_call(&METHOD_STORAGE_UPDATE_BUCKET_ACCESS_CONTROL, req, opt)
    }

    pub fn update_bucket_access_control(&self, req: &super::storage::UpdateBucketAccessControlRequest) -> ::grpcio::Result<super::storage_resources::BucketAccessControl> {
        self.update_bucket_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_bucket_access_control_async_opt(&self, req: &super::storage::UpdateBucketAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::BucketAccessControl>> {
        self.client.unary_call_async(&METHOD_STORAGE_UPDATE_BUCKET_ACCESS_CONTROL, req, opt)
    }

    pub fn update_bucket_access_control_async(&self, req: &super::storage::UpdateBucketAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::BucketAccessControl>> {
        self.update_bucket_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn patch_bucket_access_control_opt(&self, req: &super::storage::PatchBucketAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::BucketAccessControl> {
        self.client.unary_call(&METHOD_STORAGE_PATCH_BUCKET_ACCESS_CONTROL, req, opt)
    }

    pub fn patch_bucket_access_control(&self, req: &super::storage::PatchBucketAccessControlRequest) -> ::grpcio::Result<super::storage_resources::BucketAccessControl> {
        self.patch_bucket_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn patch_bucket_access_control_async_opt(&self, req: &super::storage::PatchBucketAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::BucketAccessControl>> {
        self.client.unary_call_async(&METHOD_STORAGE_PATCH_BUCKET_ACCESS_CONTROL, req, opt)
    }

    pub fn patch_bucket_access_control_async(&self, req: &super::storage::PatchBucketAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::BucketAccessControl>> {
        self.patch_bucket_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_bucket_opt(&self, req: &super::storage::DeleteBucketRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_STORAGE_DELETE_BUCKET, req, opt)
    }

    pub fn delete_bucket(&self, req: &super::storage::DeleteBucketRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_bucket_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_bucket_async_opt(&self, req: &super::storage::DeleteBucketRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_STORAGE_DELETE_BUCKET, req, opt)
    }

    pub fn delete_bucket_async(&self, req: &super::storage::DeleteBucketRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_bucket_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_bucket_opt(&self, req: &super::storage::GetBucketRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::Bucket> {
        self.client.unary_call(&METHOD_STORAGE_GET_BUCKET, req, opt)
    }

    pub fn get_bucket(&self, req: &super::storage::GetBucketRequest) -> ::grpcio::Result<super::storage_resources::Bucket> {
        self.get_bucket_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_bucket_async_opt(&self, req: &super::storage::GetBucketRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Bucket>> {
        self.client.unary_call_async(&METHOD_STORAGE_GET_BUCKET, req, opt)
    }

    pub fn get_bucket_async(&self, req: &super::storage::GetBucketRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Bucket>> {
        self.get_bucket_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn insert_bucket_opt(&self, req: &super::storage::InsertBucketRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::Bucket> {
        self.client.unary_call(&METHOD_STORAGE_INSERT_BUCKET, req, opt)
    }

    pub fn insert_bucket(&self, req: &super::storage::InsertBucketRequest) -> ::grpcio::Result<super::storage_resources::Bucket> {
        self.insert_bucket_opt(req, ::grpcio::CallOption::default())
    }

    pub fn insert_bucket_async_opt(&self, req: &super::storage::InsertBucketRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Bucket>> {
        self.client.unary_call_async(&METHOD_STORAGE_INSERT_BUCKET, req, opt)
    }

    pub fn insert_bucket_async(&self, req: &super::storage::InsertBucketRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Bucket>> {
        self.insert_bucket_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_channels_opt(&self, req: &super::storage::ListChannelsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ListChannelsResponse> {
        self.client.unary_call(&METHOD_STORAGE_LIST_CHANNELS, req, opt)
    }

    pub fn list_channels(&self, req: &super::storage::ListChannelsRequest) -> ::grpcio::Result<super::storage_resources::ListChannelsResponse> {
        self.list_channels_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_channels_async_opt(&self, req: &super::storage::ListChannelsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ListChannelsResponse>> {
        self.client.unary_call_async(&METHOD_STORAGE_LIST_CHANNELS, req, opt)
    }

    pub fn list_channels_async(&self, req: &super::storage::ListChannelsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ListChannelsResponse>> {
        self.list_channels_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_buckets_opt(&self, req: &super::storage::ListBucketsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ListBucketsResponse> {
        self.client.unary_call(&METHOD_STORAGE_LIST_BUCKETS, req, opt)
    }

    pub fn list_buckets(&self, req: &super::storage::ListBucketsRequest) -> ::grpcio::Result<super::storage_resources::ListBucketsResponse> {
        self.list_buckets_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_buckets_async_opt(&self, req: &super::storage::ListBucketsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ListBucketsResponse>> {
        self.client.unary_call_async(&METHOD_STORAGE_LIST_BUCKETS, req, opt)
    }

    pub fn list_buckets_async(&self, req: &super::storage::ListBucketsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ListBucketsResponse>> {
        self.list_buckets_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn lock_bucket_retention_policy_opt(&self, req: &super::storage::LockRetentionPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::Bucket> {
        self.client.unary_call(&METHOD_STORAGE_LOCK_BUCKET_RETENTION_POLICY, req, opt)
    }

    pub fn lock_bucket_retention_policy(&self, req: &super::storage::LockRetentionPolicyRequest) -> ::grpcio::Result<super::storage_resources::Bucket> {
        self.lock_bucket_retention_policy_opt(req, ::grpcio::CallOption::default())
    }

    pub fn lock_bucket_retention_policy_async_opt(&self, req: &super::storage::LockRetentionPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Bucket>> {
        self.client.unary_call_async(&METHOD_STORAGE_LOCK_BUCKET_RETENTION_POLICY, req, opt)
    }

    pub fn lock_bucket_retention_policy_async(&self, req: &super::storage::LockRetentionPolicyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Bucket>> {
        self.lock_bucket_retention_policy_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_bucket_iam_policy_opt(&self, req: &super::storage::GetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::policy::Policy> {
        self.client.unary_call(&METHOD_STORAGE_GET_BUCKET_IAM_POLICY, req, opt)
    }

    pub fn get_bucket_iam_policy(&self, req: &super::storage::GetIamPolicyRequest) -> ::grpcio::Result<super::policy::Policy> {
        self.get_bucket_iam_policy_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_bucket_iam_policy_async_opt(&self, req: &super::storage::GetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.client.unary_call_async(&METHOD_STORAGE_GET_BUCKET_IAM_POLICY, req, opt)
    }

    pub fn get_bucket_iam_policy_async(&self, req: &super::storage::GetIamPolicyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.get_bucket_iam_policy_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_bucket_iam_policy_opt(&self, req: &super::storage::SetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::policy::Policy> {
        self.client.unary_call(&METHOD_STORAGE_SET_BUCKET_IAM_POLICY, req, opt)
    }

    pub fn set_bucket_iam_policy(&self, req: &super::storage::SetIamPolicyRequest) -> ::grpcio::Result<super::policy::Policy> {
        self.set_bucket_iam_policy_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_bucket_iam_policy_async_opt(&self, req: &super::storage::SetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.client.unary_call_async(&METHOD_STORAGE_SET_BUCKET_IAM_POLICY, req, opt)
    }

    pub fn set_bucket_iam_policy_async(&self, req: &super::storage::SetIamPolicyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.set_bucket_iam_policy_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn test_bucket_iam_permissions_opt(&self, req: &super::storage::TestIamPermissionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::iam_policy::TestIamPermissionsResponse> {
        self.client.unary_call(&METHOD_STORAGE_TEST_BUCKET_IAM_PERMISSIONS, req, opt)
    }

    pub fn test_bucket_iam_permissions(&self, req: &super::storage::TestIamPermissionsRequest) -> ::grpcio::Result<super::iam_policy::TestIamPermissionsResponse> {
        self.test_bucket_iam_permissions_opt(req, ::grpcio::CallOption::default())
    }

    pub fn test_bucket_iam_permissions_async_opt(&self, req: &super::storage::TestIamPermissionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::iam_policy::TestIamPermissionsResponse>> {
        self.client.unary_call_async(&METHOD_STORAGE_TEST_BUCKET_IAM_PERMISSIONS, req, opt)
    }

    pub fn test_bucket_iam_permissions_async(&self, req: &super::storage::TestIamPermissionsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::iam_policy::TestIamPermissionsResponse>> {
        self.test_bucket_iam_permissions_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn patch_bucket_opt(&self, req: &super::storage::PatchBucketRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::Bucket> {
        self.client.unary_call(&METHOD_STORAGE_PATCH_BUCKET, req, opt)
    }

    pub fn patch_bucket(&self, req: &super::storage::PatchBucketRequest) -> ::grpcio::Result<super::storage_resources::Bucket> {
        self.patch_bucket_opt(req, ::grpcio::CallOption::default())
    }

    pub fn patch_bucket_async_opt(&self, req: &super::storage::PatchBucketRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Bucket>> {
        self.client.unary_call_async(&METHOD_STORAGE_PATCH_BUCKET, req, opt)
    }

    pub fn patch_bucket_async(&self, req: &super::storage::PatchBucketRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Bucket>> {
        self.patch_bucket_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_bucket_opt(&self, req: &super::storage::UpdateBucketRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::Bucket> {
        self.client.unary_call(&METHOD_STORAGE_UPDATE_BUCKET, req, opt)
    }

    pub fn update_bucket(&self, req: &super::storage::UpdateBucketRequest) -> ::grpcio::Result<super::storage_resources::Bucket> {
        self.update_bucket_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_bucket_async_opt(&self, req: &super::storage::UpdateBucketRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Bucket>> {
        self.client.unary_call_async(&METHOD_STORAGE_UPDATE_BUCKET, req, opt)
    }

    pub fn update_bucket_async(&self, req: &super::storage::UpdateBucketRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Bucket>> {
        self.update_bucket_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn stop_channel_opt(&self, req: &super::storage::StopChannelRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_STORAGE_STOP_CHANNEL, req, opt)
    }

    pub fn stop_channel(&self, req: &super::storage::StopChannelRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.stop_channel_opt(req, ::grpcio::CallOption::default())
    }

    pub fn stop_channel_async_opt(&self, req: &super::storage::StopChannelRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_STORAGE_STOP_CHANNEL, req, opt)
    }

    pub fn stop_channel_async(&self, req: &super::storage::StopChannelRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.stop_channel_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_default_object_access_control_opt(&self, req: &super::storage::DeleteDefaultObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_STORAGE_DELETE_DEFAULT_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn delete_default_object_access_control(&self, req: &super::storage::DeleteDefaultObjectAccessControlRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_default_object_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_default_object_access_control_async_opt(&self, req: &super::storage::DeleteDefaultObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_STORAGE_DELETE_DEFAULT_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn delete_default_object_access_control_async(&self, req: &super::storage::DeleteDefaultObjectAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_default_object_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_default_object_access_control_opt(&self, req: &super::storage::GetDefaultObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.client.unary_call(&METHOD_STORAGE_GET_DEFAULT_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn get_default_object_access_control(&self, req: &super::storage::GetDefaultObjectAccessControlRequest) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.get_default_object_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_default_object_access_control_async_opt(&self, req: &super::storage::GetDefaultObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.client.unary_call_async(&METHOD_STORAGE_GET_DEFAULT_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn get_default_object_access_control_async(&self, req: &super::storage::GetDefaultObjectAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.get_default_object_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn insert_default_object_access_control_opt(&self, req: &super::storage::InsertDefaultObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.client.unary_call(&METHOD_STORAGE_INSERT_DEFAULT_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn insert_default_object_access_control(&self, req: &super::storage::InsertDefaultObjectAccessControlRequest) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.insert_default_object_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn insert_default_object_access_control_async_opt(&self, req: &super::storage::InsertDefaultObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.client.unary_call_async(&METHOD_STORAGE_INSERT_DEFAULT_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn insert_default_object_access_control_async(&self, req: &super::storage::InsertDefaultObjectAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.insert_default_object_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_default_object_access_controls_opt(&self, req: &super::storage::ListDefaultObjectAccessControlsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ListObjectAccessControlsResponse> {
        self.client.unary_call(&METHOD_STORAGE_LIST_DEFAULT_OBJECT_ACCESS_CONTROLS, req, opt)
    }

    pub fn list_default_object_access_controls(&self, req: &super::storage::ListDefaultObjectAccessControlsRequest) -> ::grpcio::Result<super::storage_resources::ListObjectAccessControlsResponse> {
        self.list_default_object_access_controls_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_default_object_access_controls_async_opt(&self, req: &super::storage::ListDefaultObjectAccessControlsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ListObjectAccessControlsResponse>> {
        self.client.unary_call_async(&METHOD_STORAGE_LIST_DEFAULT_OBJECT_ACCESS_CONTROLS, req, opt)
    }

    pub fn list_default_object_access_controls_async(&self, req: &super::storage::ListDefaultObjectAccessControlsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ListObjectAccessControlsResponse>> {
        self.list_default_object_access_controls_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn patch_default_object_access_control_opt(&self, req: &super::storage::PatchDefaultObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.client.unary_call(&METHOD_STORAGE_PATCH_DEFAULT_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn patch_default_object_access_control(&self, req: &super::storage::PatchDefaultObjectAccessControlRequest) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.patch_default_object_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn patch_default_object_access_control_async_opt(&self, req: &super::storage::PatchDefaultObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.client.unary_call_async(&METHOD_STORAGE_PATCH_DEFAULT_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn patch_default_object_access_control_async(&self, req: &super::storage::PatchDefaultObjectAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.patch_default_object_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_default_object_access_control_opt(&self, req: &super::storage::UpdateDefaultObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.client.unary_call(&METHOD_STORAGE_UPDATE_DEFAULT_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn update_default_object_access_control(&self, req: &super::storage::UpdateDefaultObjectAccessControlRequest) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.update_default_object_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_default_object_access_control_async_opt(&self, req: &super::storage::UpdateDefaultObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.client.unary_call_async(&METHOD_STORAGE_UPDATE_DEFAULT_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn update_default_object_access_control_async(&self, req: &super::storage::UpdateDefaultObjectAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.update_default_object_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_notification_opt(&self, req: &super::storage::DeleteNotificationRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_STORAGE_DELETE_NOTIFICATION, req, opt)
    }

    pub fn delete_notification(&self, req: &super::storage::DeleteNotificationRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_notification_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_notification_async_opt(&self, req: &super::storage::DeleteNotificationRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_STORAGE_DELETE_NOTIFICATION, req, opt)
    }

    pub fn delete_notification_async(&self, req: &super::storage::DeleteNotificationRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_notification_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_notification_opt(&self, req: &super::storage::GetNotificationRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::Notification> {
        self.client.unary_call(&METHOD_STORAGE_GET_NOTIFICATION, req, opt)
    }

    pub fn get_notification(&self, req: &super::storage::GetNotificationRequest) -> ::grpcio::Result<super::storage_resources::Notification> {
        self.get_notification_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_notification_async_opt(&self, req: &super::storage::GetNotificationRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Notification>> {
        self.client.unary_call_async(&METHOD_STORAGE_GET_NOTIFICATION, req, opt)
    }

    pub fn get_notification_async(&self, req: &super::storage::GetNotificationRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Notification>> {
        self.get_notification_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn insert_notification_opt(&self, req: &super::storage::InsertNotificationRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::Notification> {
        self.client.unary_call(&METHOD_STORAGE_INSERT_NOTIFICATION, req, opt)
    }

    pub fn insert_notification(&self, req: &super::storage::InsertNotificationRequest) -> ::grpcio::Result<super::storage_resources::Notification> {
        self.insert_notification_opt(req, ::grpcio::CallOption::default())
    }

    pub fn insert_notification_async_opt(&self, req: &super::storage::InsertNotificationRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Notification>> {
        self.client.unary_call_async(&METHOD_STORAGE_INSERT_NOTIFICATION, req, opt)
    }

    pub fn insert_notification_async(&self, req: &super::storage::InsertNotificationRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Notification>> {
        self.insert_notification_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_notifications_opt(&self, req: &super::storage::ListNotificationsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ListNotificationsResponse> {
        self.client.unary_call(&METHOD_STORAGE_LIST_NOTIFICATIONS, req, opt)
    }

    pub fn list_notifications(&self, req: &super::storage::ListNotificationsRequest) -> ::grpcio::Result<super::storage_resources::ListNotificationsResponse> {
        self.list_notifications_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_notifications_async_opt(&self, req: &super::storage::ListNotificationsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ListNotificationsResponse>> {
        self.client.unary_call_async(&METHOD_STORAGE_LIST_NOTIFICATIONS, req, opt)
    }

    pub fn list_notifications_async(&self, req: &super::storage::ListNotificationsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ListNotificationsResponse>> {
        self.list_notifications_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_object_access_control_opt(&self, req: &super::storage::DeleteObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_STORAGE_DELETE_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn delete_object_access_control(&self, req: &super::storage::DeleteObjectAccessControlRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_object_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_object_access_control_async_opt(&self, req: &super::storage::DeleteObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_STORAGE_DELETE_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn delete_object_access_control_async(&self, req: &super::storage::DeleteObjectAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_object_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_object_access_control_opt(&self, req: &super::storage::GetObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.client.unary_call(&METHOD_STORAGE_GET_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn get_object_access_control(&self, req: &super::storage::GetObjectAccessControlRequest) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.get_object_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_object_access_control_async_opt(&self, req: &super::storage::GetObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.client.unary_call_async(&METHOD_STORAGE_GET_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn get_object_access_control_async(&self, req: &super::storage::GetObjectAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.get_object_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn insert_object_access_control_opt(&self, req: &super::storage::InsertObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.client.unary_call(&METHOD_STORAGE_INSERT_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn insert_object_access_control(&self, req: &super::storage::InsertObjectAccessControlRequest) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.insert_object_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn insert_object_access_control_async_opt(&self, req: &super::storage::InsertObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.client.unary_call_async(&METHOD_STORAGE_INSERT_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn insert_object_access_control_async(&self, req: &super::storage::InsertObjectAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.insert_object_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_object_access_controls_opt(&self, req: &super::storage::ListObjectAccessControlsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ListObjectAccessControlsResponse> {
        self.client.unary_call(&METHOD_STORAGE_LIST_OBJECT_ACCESS_CONTROLS, req, opt)
    }

    pub fn list_object_access_controls(&self, req: &super::storage::ListObjectAccessControlsRequest) -> ::grpcio::Result<super::storage_resources::ListObjectAccessControlsResponse> {
        self.list_object_access_controls_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_object_access_controls_async_opt(&self, req: &super::storage::ListObjectAccessControlsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ListObjectAccessControlsResponse>> {
        self.client.unary_call_async(&METHOD_STORAGE_LIST_OBJECT_ACCESS_CONTROLS, req, opt)
    }

    pub fn list_object_access_controls_async(&self, req: &super::storage::ListObjectAccessControlsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ListObjectAccessControlsResponse>> {
        self.list_object_access_controls_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn patch_object_access_control_opt(&self, req: &super::storage::PatchObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.client.unary_call(&METHOD_STORAGE_PATCH_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn patch_object_access_control(&self, req: &super::storage::PatchObjectAccessControlRequest) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.patch_object_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn patch_object_access_control_async_opt(&self, req: &super::storage::PatchObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.client.unary_call_async(&METHOD_STORAGE_PATCH_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn patch_object_access_control_async(&self, req: &super::storage::PatchObjectAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.patch_object_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_object_access_control_opt(&self, req: &super::storage::UpdateObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.client.unary_call(&METHOD_STORAGE_UPDATE_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn update_object_access_control(&self, req: &super::storage::UpdateObjectAccessControlRequest) -> ::grpcio::Result<super::storage_resources::ObjectAccessControl> {
        self.update_object_access_control_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_object_access_control_async_opt(&self, req: &super::storage::UpdateObjectAccessControlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.client.unary_call_async(&METHOD_STORAGE_UPDATE_OBJECT_ACCESS_CONTROL, req, opt)
    }

    pub fn update_object_access_control_async(&self, req: &super::storage::UpdateObjectAccessControlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ObjectAccessControl>> {
        self.update_object_access_control_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn compose_object_opt(&self, req: &super::storage::ComposeObjectRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::Object> {
        self.client.unary_call(&METHOD_STORAGE_COMPOSE_OBJECT, req, opt)
    }

    pub fn compose_object(&self, req: &super::storage::ComposeObjectRequest) -> ::grpcio::Result<super::storage_resources::Object> {
        self.compose_object_opt(req, ::grpcio::CallOption::default())
    }

    pub fn compose_object_async_opt(&self, req: &super::storage::ComposeObjectRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Object>> {
        self.client.unary_call_async(&METHOD_STORAGE_COMPOSE_OBJECT, req, opt)
    }

    pub fn compose_object_async(&self, req: &super::storage::ComposeObjectRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Object>> {
        self.compose_object_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn copy_object_opt(&self, req: &super::storage::CopyObjectRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::Object> {
        self.client.unary_call(&METHOD_STORAGE_COPY_OBJECT, req, opt)
    }

    pub fn copy_object(&self, req: &super::storage::CopyObjectRequest) -> ::grpcio::Result<super::storage_resources::Object> {
        self.copy_object_opt(req, ::grpcio::CallOption::default())
    }

    pub fn copy_object_async_opt(&self, req: &super::storage::CopyObjectRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Object>> {
        self.client.unary_call_async(&METHOD_STORAGE_COPY_OBJECT, req, opt)
    }

    pub fn copy_object_async(&self, req: &super::storage::CopyObjectRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Object>> {
        self.copy_object_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_object_opt(&self, req: &super::storage::DeleteObjectRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_STORAGE_DELETE_OBJECT, req, opt)
    }

    pub fn delete_object(&self, req: &super::storage::DeleteObjectRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_object_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_object_async_opt(&self, req: &super::storage::DeleteObjectRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_STORAGE_DELETE_OBJECT, req, opt)
    }

    pub fn delete_object_async(&self, req: &super::storage::DeleteObjectRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_object_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_object_opt(&self, req: &super::storage::GetObjectRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::Object> {
        self.client.unary_call(&METHOD_STORAGE_GET_OBJECT, req, opt)
    }

    pub fn get_object(&self, req: &super::storage::GetObjectRequest) -> ::grpcio::Result<super::storage_resources::Object> {
        self.get_object_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_object_async_opt(&self, req: &super::storage::GetObjectRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Object>> {
        self.client.unary_call_async(&METHOD_STORAGE_GET_OBJECT, req, opt)
    }

    pub fn get_object_async(&self, req: &super::storage::GetObjectRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Object>> {
        self.get_object_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_object_media_opt(&self, req: &super::storage::GetObjectMediaRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::storage::GetObjectMediaResponse>> {
        self.client.server_streaming(&METHOD_STORAGE_GET_OBJECT_MEDIA, req, opt)
    }

    pub fn get_object_media(&self, req: &super::storage::GetObjectMediaRequest) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::storage::GetObjectMediaResponse>> {
        self.get_object_media_opt(req, ::grpcio::CallOption::default())
    }

    pub fn insert_object_opt(&self, opt: ::grpcio::CallOption) -> ::grpcio::Result<(::grpcio::ClientCStreamSender<super::storage::InsertObjectRequest>, ::grpcio::ClientCStreamReceiver<super::storage_resources::Object>)> {
        self.client.client_streaming(&METHOD_STORAGE_INSERT_OBJECT, opt)
    }

    pub fn insert_object(&self) -> ::grpcio::Result<(::grpcio::ClientCStreamSender<super::storage::InsertObjectRequest>, ::grpcio::ClientCStreamReceiver<super::storage_resources::Object>)> {
        self.insert_object_opt(::grpcio::CallOption::default())
    }

    pub fn list_objects_opt(&self, req: &super::storage::ListObjectsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ListObjectsResponse> {
        self.client.unary_call(&METHOD_STORAGE_LIST_OBJECTS, req, opt)
    }

    pub fn list_objects(&self, req: &super::storage::ListObjectsRequest) -> ::grpcio::Result<super::storage_resources::ListObjectsResponse> {
        self.list_objects_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_objects_async_opt(&self, req: &super::storage::ListObjectsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ListObjectsResponse>> {
        self.client.unary_call_async(&METHOD_STORAGE_LIST_OBJECTS, req, opt)
    }

    pub fn list_objects_async(&self, req: &super::storage::ListObjectsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ListObjectsResponse>> {
        self.list_objects_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn rewrite_object_opt(&self, req: &super::storage::RewriteObjectRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage::RewriteResponse> {
        self.client.unary_call(&METHOD_STORAGE_REWRITE_OBJECT, req, opt)
    }

    pub fn rewrite_object(&self, req: &super::storage::RewriteObjectRequest) -> ::grpcio::Result<super::storage::RewriteResponse> {
        self.rewrite_object_opt(req, ::grpcio::CallOption::default())
    }

    pub fn rewrite_object_async_opt(&self, req: &super::storage::RewriteObjectRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage::RewriteResponse>> {
        self.client.unary_call_async(&METHOD_STORAGE_REWRITE_OBJECT, req, opt)
    }

    pub fn rewrite_object_async(&self, req: &super::storage::RewriteObjectRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage::RewriteResponse>> {
        self.rewrite_object_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn start_resumable_write_opt(&self, req: &super::storage::StartResumableWriteRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage::StartResumableWriteResponse> {
        self.client.unary_call(&METHOD_STORAGE_START_RESUMABLE_WRITE, req, opt)
    }

    pub fn start_resumable_write(&self, req: &super::storage::StartResumableWriteRequest) -> ::grpcio::Result<super::storage::StartResumableWriteResponse> {
        self.start_resumable_write_opt(req, ::grpcio::CallOption::default())
    }

    pub fn start_resumable_write_async_opt(&self, req: &super::storage::StartResumableWriteRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage::StartResumableWriteResponse>> {
        self.client.unary_call_async(&METHOD_STORAGE_START_RESUMABLE_WRITE, req, opt)
    }

    pub fn start_resumable_write_async(&self, req: &super::storage::StartResumableWriteRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage::StartResumableWriteResponse>> {
        self.start_resumable_write_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn query_write_status_opt(&self, req: &super::storage::QueryWriteStatusRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage::QueryWriteStatusResponse> {
        self.client.unary_call(&METHOD_STORAGE_QUERY_WRITE_STATUS, req, opt)
    }

    pub fn query_write_status(&self, req: &super::storage::QueryWriteStatusRequest) -> ::grpcio::Result<super::storage::QueryWriteStatusResponse> {
        self.query_write_status_opt(req, ::grpcio::CallOption::default())
    }

    pub fn query_write_status_async_opt(&self, req: &super::storage::QueryWriteStatusRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage::QueryWriteStatusResponse>> {
        self.client.unary_call_async(&METHOD_STORAGE_QUERY_WRITE_STATUS, req, opt)
    }

    pub fn query_write_status_async(&self, req: &super::storage::QueryWriteStatusRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage::QueryWriteStatusResponse>> {
        self.query_write_status_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn patch_object_opt(&self, req: &super::storage::PatchObjectRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::Object> {
        self.client.unary_call(&METHOD_STORAGE_PATCH_OBJECT, req, opt)
    }

    pub fn patch_object(&self, req: &super::storage::PatchObjectRequest) -> ::grpcio::Result<super::storage_resources::Object> {
        self.patch_object_opt(req, ::grpcio::CallOption::default())
    }

    pub fn patch_object_async_opt(&self, req: &super::storage::PatchObjectRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Object>> {
        self.client.unary_call_async(&METHOD_STORAGE_PATCH_OBJECT, req, opt)
    }

    pub fn patch_object_async(&self, req: &super::storage::PatchObjectRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Object>> {
        self.patch_object_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_object_opt(&self, req: &super::storage::UpdateObjectRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::Object> {
        self.client.unary_call(&METHOD_STORAGE_UPDATE_OBJECT, req, opt)
    }

    pub fn update_object(&self, req: &super::storage::UpdateObjectRequest) -> ::grpcio::Result<super::storage_resources::Object> {
        self.update_object_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_object_async_opt(&self, req: &super::storage::UpdateObjectRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Object>> {
        self.client.unary_call_async(&METHOD_STORAGE_UPDATE_OBJECT, req, opt)
    }

    pub fn update_object_async(&self, req: &super::storage::UpdateObjectRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Object>> {
        self.update_object_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_object_iam_policy_opt(&self, req: &super::storage::GetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::policy::Policy> {
        self.client.unary_call(&METHOD_STORAGE_GET_OBJECT_IAM_POLICY, req, opt)
    }

    pub fn get_object_iam_policy(&self, req: &super::storage::GetIamPolicyRequest) -> ::grpcio::Result<super::policy::Policy> {
        self.get_object_iam_policy_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_object_iam_policy_async_opt(&self, req: &super::storage::GetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.client.unary_call_async(&METHOD_STORAGE_GET_OBJECT_IAM_POLICY, req, opt)
    }

    pub fn get_object_iam_policy_async(&self, req: &super::storage::GetIamPolicyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.get_object_iam_policy_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_object_iam_policy_opt(&self, req: &super::storage::SetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::policy::Policy> {
        self.client.unary_call(&METHOD_STORAGE_SET_OBJECT_IAM_POLICY, req, opt)
    }

    pub fn set_object_iam_policy(&self, req: &super::storage::SetIamPolicyRequest) -> ::grpcio::Result<super::policy::Policy> {
        self.set_object_iam_policy_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_object_iam_policy_async_opt(&self, req: &super::storage::SetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.client.unary_call_async(&METHOD_STORAGE_SET_OBJECT_IAM_POLICY, req, opt)
    }

    pub fn set_object_iam_policy_async(&self, req: &super::storage::SetIamPolicyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.set_object_iam_policy_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn test_object_iam_permissions_opt(&self, req: &super::storage::TestIamPermissionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::iam_policy::TestIamPermissionsResponse> {
        self.client.unary_call(&METHOD_STORAGE_TEST_OBJECT_IAM_PERMISSIONS, req, opt)
    }

    pub fn test_object_iam_permissions(&self, req: &super::storage::TestIamPermissionsRequest) -> ::grpcio::Result<super::iam_policy::TestIamPermissionsResponse> {
        self.test_object_iam_permissions_opt(req, ::grpcio::CallOption::default())
    }

    pub fn test_object_iam_permissions_async_opt(&self, req: &super::storage::TestIamPermissionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::iam_policy::TestIamPermissionsResponse>> {
        self.client.unary_call_async(&METHOD_STORAGE_TEST_OBJECT_IAM_PERMISSIONS, req, opt)
    }

    pub fn test_object_iam_permissions_async(&self, req: &super::storage::TestIamPermissionsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::iam_policy::TestIamPermissionsResponse>> {
        self.test_object_iam_permissions_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn watch_all_objects_opt(&self, req: &super::storage::WatchAllObjectsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::Channel> {
        self.client.unary_call(&METHOD_STORAGE_WATCH_ALL_OBJECTS, req, opt)
    }

    pub fn watch_all_objects(&self, req: &super::storage::WatchAllObjectsRequest) -> ::grpcio::Result<super::storage_resources::Channel> {
        self.watch_all_objects_opt(req, ::grpcio::CallOption::default())
    }

    pub fn watch_all_objects_async_opt(&self, req: &super::storage::WatchAllObjectsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Channel>> {
        self.client.unary_call_async(&METHOD_STORAGE_WATCH_ALL_OBJECTS, req, opt)
    }

    pub fn watch_all_objects_async(&self, req: &super::storage::WatchAllObjectsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::Channel>> {
        self.watch_all_objects_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_service_account_opt(&self, req: &super::storage::GetProjectServiceAccountRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::ServiceAccount> {
        self.client.unary_call(&METHOD_STORAGE_GET_SERVICE_ACCOUNT, req, opt)
    }

    pub fn get_service_account(&self, req: &super::storage::GetProjectServiceAccountRequest) -> ::grpcio::Result<super::storage_resources::ServiceAccount> {
        self.get_service_account_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_service_account_async_opt(&self, req: &super::storage::GetProjectServiceAccountRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ServiceAccount>> {
        self.client.unary_call_async(&METHOD_STORAGE_GET_SERVICE_ACCOUNT, req, opt)
    }

    pub fn get_service_account_async(&self, req: &super::storage::GetProjectServiceAccountRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::ServiceAccount>> {
        self.get_service_account_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_hmac_key_opt(&self, req: &super::storage::CreateHmacKeyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage::CreateHmacKeyResponse> {
        self.client.unary_call(&METHOD_STORAGE_CREATE_HMAC_KEY, req, opt)
    }

    pub fn create_hmac_key(&self, req: &super::storage::CreateHmacKeyRequest) -> ::grpcio::Result<super::storage::CreateHmacKeyResponse> {
        self.create_hmac_key_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_hmac_key_async_opt(&self, req: &super::storage::CreateHmacKeyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage::CreateHmacKeyResponse>> {
        self.client.unary_call_async(&METHOD_STORAGE_CREATE_HMAC_KEY, req, opt)
    }

    pub fn create_hmac_key_async(&self, req: &super::storage::CreateHmacKeyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage::CreateHmacKeyResponse>> {
        self.create_hmac_key_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_hmac_key_opt(&self, req: &super::storage::DeleteHmacKeyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_STORAGE_DELETE_HMAC_KEY, req, opt)
    }

    pub fn delete_hmac_key(&self, req: &super::storage::DeleteHmacKeyRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_hmac_key_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_hmac_key_async_opt(&self, req: &super::storage::DeleteHmacKeyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_STORAGE_DELETE_HMAC_KEY, req, opt)
    }

    pub fn delete_hmac_key_async(&self, req: &super::storage::DeleteHmacKeyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_hmac_key_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_hmac_key_opt(&self, req: &super::storage::GetHmacKeyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::HmacKeyMetadata> {
        self.client.unary_call(&METHOD_STORAGE_GET_HMAC_KEY, req, opt)
    }

    pub fn get_hmac_key(&self, req: &super::storage::GetHmacKeyRequest) -> ::grpcio::Result<super::storage_resources::HmacKeyMetadata> {
        self.get_hmac_key_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_hmac_key_async_opt(&self, req: &super::storage::GetHmacKeyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::HmacKeyMetadata>> {
        self.client.unary_call_async(&METHOD_STORAGE_GET_HMAC_KEY, req, opt)
    }

    pub fn get_hmac_key_async(&self, req: &super::storage::GetHmacKeyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::HmacKeyMetadata>> {
        self.get_hmac_key_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_hmac_keys_opt(&self, req: &super::storage::ListHmacKeysRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage::ListHmacKeysResponse> {
        self.client.unary_call(&METHOD_STORAGE_LIST_HMAC_KEYS, req, opt)
    }

    pub fn list_hmac_keys(&self, req: &super::storage::ListHmacKeysRequest) -> ::grpcio::Result<super::storage::ListHmacKeysResponse> {
        self.list_hmac_keys_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_hmac_keys_async_opt(&self, req: &super::storage::ListHmacKeysRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage::ListHmacKeysResponse>> {
        self.client.unary_call_async(&METHOD_STORAGE_LIST_HMAC_KEYS, req, opt)
    }

    pub fn list_hmac_keys_async(&self, req: &super::storage::ListHmacKeysRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage::ListHmacKeysResponse>> {
        self.list_hmac_keys_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_hmac_key_opt(&self, req: &super::storage::UpdateHmacKeyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::storage_resources::HmacKeyMetadata> {
        self.client.unary_call(&METHOD_STORAGE_UPDATE_HMAC_KEY, req, opt)
    }

    pub fn update_hmac_key(&self, req: &super::storage::UpdateHmacKeyRequest) -> ::grpcio::Result<super::storage_resources::HmacKeyMetadata> {
        self.update_hmac_key_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_hmac_key_async_opt(&self, req: &super::storage::UpdateHmacKeyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::HmacKeyMetadata>> {
        self.client.unary_call_async(&METHOD_STORAGE_UPDATE_HMAC_KEY, req, opt)
    }

    pub fn update_hmac_key_async(&self, req: &super::storage::UpdateHmacKeyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::storage_resources::HmacKeyMetadata>> {
        self.update_hmac_key_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Output = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait Storage {
    fn delete_bucket_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::DeleteBucketAccessControlRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn get_bucket_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::GetBucketAccessControlRequest, sink: ::grpcio::UnarySink<super::storage_resources::BucketAccessControl>);
    fn insert_bucket_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::InsertBucketAccessControlRequest, sink: ::grpcio::UnarySink<super::storage_resources::BucketAccessControl>);
    fn list_bucket_access_controls(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::ListBucketAccessControlsRequest, sink: ::grpcio::UnarySink<super::storage_resources::ListBucketAccessControlsResponse>);
    fn update_bucket_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::UpdateBucketAccessControlRequest, sink: ::grpcio::UnarySink<super::storage_resources::BucketAccessControl>);
    fn patch_bucket_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::PatchBucketAccessControlRequest, sink: ::grpcio::UnarySink<super::storage_resources::BucketAccessControl>);
    fn delete_bucket(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::DeleteBucketRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn get_bucket(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::GetBucketRequest, sink: ::grpcio::UnarySink<super::storage_resources::Bucket>);
    fn insert_bucket(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::InsertBucketRequest, sink: ::grpcio::UnarySink<super::storage_resources::Bucket>);
    fn list_channels(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::ListChannelsRequest, sink: ::grpcio::UnarySink<super::storage_resources::ListChannelsResponse>);
    fn list_buckets(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::ListBucketsRequest, sink: ::grpcio::UnarySink<super::storage_resources::ListBucketsResponse>);
    fn lock_bucket_retention_policy(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::LockRetentionPolicyRequest, sink: ::grpcio::UnarySink<super::storage_resources::Bucket>);
    fn get_bucket_iam_policy(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::GetIamPolicyRequest, sink: ::grpcio::UnarySink<super::policy::Policy>);
    fn set_bucket_iam_policy(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::SetIamPolicyRequest, sink: ::grpcio::UnarySink<super::policy::Policy>);
    fn test_bucket_iam_permissions(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::TestIamPermissionsRequest, sink: ::grpcio::UnarySink<super::iam_policy::TestIamPermissionsResponse>);
    fn patch_bucket(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::PatchBucketRequest, sink: ::grpcio::UnarySink<super::storage_resources::Bucket>);
    fn update_bucket(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::UpdateBucketRequest, sink: ::grpcio::UnarySink<super::storage_resources::Bucket>);
    fn stop_channel(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::StopChannelRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn delete_default_object_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::DeleteDefaultObjectAccessControlRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn get_default_object_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::GetDefaultObjectAccessControlRequest, sink: ::grpcio::UnarySink<super::storage_resources::ObjectAccessControl>);
    fn insert_default_object_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::InsertDefaultObjectAccessControlRequest, sink: ::grpcio::UnarySink<super::storage_resources::ObjectAccessControl>);
    fn list_default_object_access_controls(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::ListDefaultObjectAccessControlsRequest, sink: ::grpcio::UnarySink<super::storage_resources::ListObjectAccessControlsResponse>);
    fn patch_default_object_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::PatchDefaultObjectAccessControlRequest, sink: ::grpcio::UnarySink<super::storage_resources::ObjectAccessControl>);
    fn update_default_object_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::UpdateDefaultObjectAccessControlRequest, sink: ::grpcio::UnarySink<super::storage_resources::ObjectAccessControl>);
    fn delete_notification(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::DeleteNotificationRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn get_notification(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::GetNotificationRequest, sink: ::grpcio::UnarySink<super::storage_resources::Notification>);
    fn insert_notification(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::InsertNotificationRequest, sink: ::grpcio::UnarySink<super::storage_resources::Notification>);
    fn list_notifications(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::ListNotificationsRequest, sink: ::grpcio::UnarySink<super::storage_resources::ListNotificationsResponse>);
    fn delete_object_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::DeleteObjectAccessControlRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn get_object_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::GetObjectAccessControlRequest, sink: ::grpcio::UnarySink<super::storage_resources::ObjectAccessControl>);
    fn insert_object_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::InsertObjectAccessControlRequest, sink: ::grpcio::UnarySink<super::storage_resources::ObjectAccessControl>);
    fn list_object_access_controls(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::ListObjectAccessControlsRequest, sink: ::grpcio::UnarySink<super::storage_resources::ListObjectAccessControlsResponse>);
    fn patch_object_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::PatchObjectAccessControlRequest, sink: ::grpcio::UnarySink<super::storage_resources::ObjectAccessControl>);
    fn update_object_access_control(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::UpdateObjectAccessControlRequest, sink: ::grpcio::UnarySink<super::storage_resources::ObjectAccessControl>);
    fn compose_object(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::ComposeObjectRequest, sink: ::grpcio::UnarySink<super::storage_resources::Object>);
    fn copy_object(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::CopyObjectRequest, sink: ::grpcio::UnarySink<super::storage_resources::Object>);
    fn delete_object(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::DeleteObjectRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn get_object(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::GetObjectRequest, sink: ::grpcio::UnarySink<super::storage_resources::Object>);
    fn get_object_media(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::GetObjectMediaRequest, sink: ::grpcio::ServerStreamingSink<super::storage::GetObjectMediaResponse>);
    fn insert_object(&mut self, ctx: ::grpcio::RpcContext, stream: ::grpcio::RequestStream<super::storage::InsertObjectRequest>, sink: ::grpcio::ClientStreamingSink<super::storage_resources::Object>);
    fn list_objects(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::ListObjectsRequest, sink: ::grpcio::UnarySink<super::storage_resources::ListObjectsResponse>);
    fn rewrite_object(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::RewriteObjectRequest, sink: ::grpcio::UnarySink<super::storage::RewriteResponse>);
    fn start_resumable_write(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::StartResumableWriteRequest, sink: ::grpcio::UnarySink<super::storage::StartResumableWriteResponse>);
    fn query_write_status(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::QueryWriteStatusRequest, sink: ::grpcio::UnarySink<super::storage::QueryWriteStatusResponse>);
    fn patch_object(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::PatchObjectRequest, sink: ::grpcio::UnarySink<super::storage_resources::Object>);
    fn update_object(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::UpdateObjectRequest, sink: ::grpcio::UnarySink<super::storage_resources::Object>);
    fn get_object_iam_policy(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::GetIamPolicyRequest, sink: ::grpcio::UnarySink<super::policy::Policy>);
    fn set_object_iam_policy(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::SetIamPolicyRequest, sink: ::grpcio::UnarySink<super::policy::Policy>);
    fn test_object_iam_permissions(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::TestIamPermissionsRequest, sink: ::grpcio::UnarySink<super::iam_policy::TestIamPermissionsResponse>);
    fn watch_all_objects(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::WatchAllObjectsRequest, sink: ::grpcio::UnarySink<super::storage_resources::Channel>);
    fn get_service_account(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::GetProjectServiceAccountRequest, sink: ::grpcio::UnarySink<super::storage_resources::ServiceAccount>);
    fn create_hmac_key(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::CreateHmacKeyRequest, sink: ::grpcio::UnarySink<super::storage::CreateHmacKeyResponse>);
    fn delete_hmac_key(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::DeleteHmacKeyRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn get_hmac_key(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::GetHmacKeyRequest, sink: ::grpcio::UnarySink<super::storage_resources::HmacKeyMetadata>);
    fn list_hmac_keys(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::ListHmacKeysRequest, sink: ::grpcio::UnarySink<super::storage::ListHmacKeysResponse>);
    fn update_hmac_key(&mut self, ctx: ::grpcio::RpcContext, req: super::storage::UpdateHmacKeyRequest, sink: ::grpcio::UnarySink<super::storage_resources::HmacKeyMetadata>);
}

pub fn create_storage<S: Storage + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_DELETE_BUCKET_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.delete_bucket_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_GET_BUCKET_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.get_bucket_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_INSERT_BUCKET_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.insert_bucket_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_LIST_BUCKET_ACCESS_CONTROLS, move |ctx, req, resp| {
        instance.list_bucket_access_controls(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_UPDATE_BUCKET_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.update_bucket_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_PATCH_BUCKET_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.patch_bucket_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_DELETE_BUCKET, move |ctx, req, resp| {
        instance.delete_bucket(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_GET_BUCKET, move |ctx, req, resp| {
        instance.get_bucket(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_INSERT_BUCKET, move |ctx, req, resp| {
        instance.insert_bucket(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_LIST_CHANNELS, move |ctx, req, resp| {
        instance.list_channels(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_LIST_BUCKETS, move |ctx, req, resp| {
        instance.list_buckets(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_LOCK_BUCKET_RETENTION_POLICY, move |ctx, req, resp| {
        instance.lock_bucket_retention_policy(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_GET_BUCKET_IAM_POLICY, move |ctx, req, resp| {
        instance.get_bucket_iam_policy(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_SET_BUCKET_IAM_POLICY, move |ctx, req, resp| {
        instance.set_bucket_iam_policy(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_TEST_BUCKET_IAM_PERMISSIONS, move |ctx, req, resp| {
        instance.test_bucket_iam_permissions(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_PATCH_BUCKET, move |ctx, req, resp| {
        instance.patch_bucket(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_UPDATE_BUCKET, move |ctx, req, resp| {
        instance.update_bucket(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_STOP_CHANNEL, move |ctx, req, resp| {
        instance.stop_channel(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_DELETE_DEFAULT_OBJECT_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.delete_default_object_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_GET_DEFAULT_OBJECT_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.get_default_object_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_INSERT_DEFAULT_OBJECT_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.insert_default_object_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_LIST_DEFAULT_OBJECT_ACCESS_CONTROLS, move |ctx, req, resp| {
        instance.list_default_object_access_controls(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_PATCH_DEFAULT_OBJECT_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.patch_default_object_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_UPDATE_DEFAULT_OBJECT_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.update_default_object_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_DELETE_NOTIFICATION, move |ctx, req, resp| {
        instance.delete_notification(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_GET_NOTIFICATION, move |ctx, req, resp| {
        instance.get_notification(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_INSERT_NOTIFICATION, move |ctx, req, resp| {
        instance.insert_notification(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_LIST_NOTIFICATIONS, move |ctx, req, resp| {
        instance.list_notifications(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_DELETE_OBJECT_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.delete_object_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_GET_OBJECT_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.get_object_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_INSERT_OBJECT_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.insert_object_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_LIST_OBJECT_ACCESS_CONTROLS, move |ctx, req, resp| {
        instance.list_object_access_controls(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_PATCH_OBJECT_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.patch_object_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_UPDATE_OBJECT_ACCESS_CONTROL, move |ctx, req, resp| {
        instance.update_object_access_control(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_COMPOSE_OBJECT, move |ctx, req, resp| {
        instance.compose_object(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_COPY_OBJECT, move |ctx, req, resp| {
        instance.copy_object(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_DELETE_OBJECT, move |ctx, req, resp| {
        instance.delete_object(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_GET_OBJECT, move |ctx, req, resp| {
        instance.get_object(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_server_streaming_handler(&METHOD_STORAGE_GET_OBJECT_MEDIA, move |ctx, req, resp| {
        instance.get_object_media(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_client_streaming_handler(&METHOD_STORAGE_INSERT_OBJECT, move |ctx, req, resp| {
        instance.insert_object(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_LIST_OBJECTS, move |ctx, req, resp| {
        instance.list_objects(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_REWRITE_OBJECT, move |ctx, req, resp| {
        instance.rewrite_object(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_START_RESUMABLE_WRITE, move |ctx, req, resp| {
        instance.start_resumable_write(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_QUERY_WRITE_STATUS, move |ctx, req, resp| {
        instance.query_write_status(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_PATCH_OBJECT, move |ctx, req, resp| {
        instance.patch_object(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_UPDATE_OBJECT, move |ctx, req, resp| {
        instance.update_object(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_GET_OBJECT_IAM_POLICY, move |ctx, req, resp| {
        instance.get_object_iam_policy(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_SET_OBJECT_IAM_POLICY, move |ctx, req, resp| {
        instance.set_object_iam_policy(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_TEST_OBJECT_IAM_PERMISSIONS, move |ctx, req, resp| {
        instance.test_object_iam_permissions(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_WATCH_ALL_OBJECTS, move |ctx, req, resp| {
        instance.watch_all_objects(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_GET_SERVICE_ACCOUNT, move |ctx, req, resp| {
        instance.get_service_account(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_CREATE_HMAC_KEY, move |ctx, req, resp| {
        instance.create_hmac_key(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_DELETE_HMAC_KEY, move |ctx, req, resp| {
        instance.delete_hmac_key(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_GET_HMAC_KEY, move |ctx, req, resp| {
        instance.get_hmac_key(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_STORAGE_LIST_HMAC_KEYS, move |ctx, req, resp| {
        instance.list_hmac_keys(ctx, req, resp)
    });
    let mut instance = s;
    builder = builder.add_unary_handler(&METHOD_STORAGE_UPDATE_HMAC_KEY, move |ctx, req, resp| {
        instance.update_hmac_key(ctx, req, resp)
    });
    builder.build()
}
