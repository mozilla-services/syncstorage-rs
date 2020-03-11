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

const METHOD_BIGTABLE_INSTANCE_ADMIN_CREATE_INSTANCE: ::grpcio::Method<super::bigtable_instance_admin::CreateInstanceRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/CreateInstance",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_GET_INSTANCE: ::grpcio::Method<super::bigtable_instance_admin::GetInstanceRequest, super::instance::Instance> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/GetInstance",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_LIST_INSTANCES: ::grpcio::Method<super::bigtable_instance_admin::ListInstancesRequest, super::bigtable_instance_admin::ListInstancesResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/ListInstances",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_UPDATE_INSTANCE: ::grpcio::Method<super::instance::Instance, super::instance::Instance> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/UpdateInstance",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_PARTIAL_UPDATE_INSTANCE: ::grpcio::Method<super::bigtable_instance_admin::PartialUpdateInstanceRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/PartialUpdateInstance",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_DELETE_INSTANCE: ::grpcio::Method<super::bigtable_instance_admin::DeleteInstanceRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/DeleteInstance",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_CREATE_CLUSTER: ::grpcio::Method<super::bigtable_instance_admin::CreateClusterRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/CreateCluster",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_GET_CLUSTER: ::grpcio::Method<super::bigtable_instance_admin::GetClusterRequest, super::instance::Cluster> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/GetCluster",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_LIST_CLUSTERS: ::grpcio::Method<super::bigtable_instance_admin::ListClustersRequest, super::bigtable_instance_admin::ListClustersResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/ListClusters",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_UPDATE_CLUSTER: ::grpcio::Method<super::instance::Cluster, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/UpdateCluster",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_DELETE_CLUSTER: ::grpcio::Method<super::bigtable_instance_admin::DeleteClusterRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/DeleteCluster",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_CREATE_APP_PROFILE: ::grpcio::Method<super::bigtable_instance_admin::CreateAppProfileRequest, super::instance::AppProfile> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/CreateAppProfile",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_GET_APP_PROFILE: ::grpcio::Method<super::bigtable_instance_admin::GetAppProfileRequest, super::instance::AppProfile> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/GetAppProfile",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_LIST_APP_PROFILES: ::grpcio::Method<super::bigtable_instance_admin::ListAppProfilesRequest, super::bigtable_instance_admin::ListAppProfilesResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/ListAppProfiles",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_UPDATE_APP_PROFILE: ::grpcio::Method<super::bigtable_instance_admin::UpdateAppProfileRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/UpdateAppProfile",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_DELETE_APP_PROFILE: ::grpcio::Method<super::bigtable_instance_admin::DeleteAppProfileRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/DeleteAppProfile",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_GET_IAM_POLICY: ::grpcio::Method<super::iam_policy::GetIamPolicyRequest, super::policy::Policy> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/GetIamPolicy",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_SET_IAM_POLICY: ::grpcio::Method<super::iam_policy::SetIamPolicyRequest, super::policy::Policy> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/SetIamPolicy",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_INSTANCE_ADMIN_TEST_IAM_PERMISSIONS: ::grpcio::Method<super::iam_policy::TestIamPermissionsRequest, super::iam_policy::TestIamPermissionsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableInstanceAdmin/TestIamPermissions",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct BigtableInstanceAdminClient {
    client: ::grpcio::Client,
}

impl BigtableInstanceAdminClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        BigtableInstanceAdminClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn create_instance_opt(&self, req: &super::bigtable_instance_admin::CreateInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_CREATE_INSTANCE, req, opt)
    }

    pub fn create_instance(&self, req: &super::bigtable_instance_admin::CreateInstanceRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.create_instance_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_instance_async_opt(&self, req: &super::bigtable_instance_admin::CreateInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_CREATE_INSTANCE, req, opt)
    }

    pub fn create_instance_async(&self, req: &super::bigtable_instance_admin::CreateInstanceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.create_instance_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_instance_opt(&self, req: &super::bigtable_instance_admin::GetInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::instance::Instance> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_GET_INSTANCE, req, opt)
    }

    pub fn get_instance(&self, req: &super::bigtable_instance_admin::GetInstanceRequest) -> ::grpcio::Result<super::instance::Instance> {
        self.get_instance_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_instance_async_opt(&self, req: &super::bigtable_instance_admin::GetInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::instance::Instance>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_GET_INSTANCE, req, opt)
    }

    pub fn get_instance_async(&self, req: &super::bigtable_instance_admin::GetInstanceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::instance::Instance>> {
        self.get_instance_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_instances_opt(&self, req: &super::bigtable_instance_admin::ListInstancesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_instance_admin::ListInstancesResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_LIST_INSTANCES, req, opt)
    }

    pub fn list_instances(&self, req: &super::bigtable_instance_admin::ListInstancesRequest) -> ::grpcio::Result<super::bigtable_instance_admin::ListInstancesResponse> {
        self.list_instances_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_instances_async_opt(&self, req: &super::bigtable_instance_admin::ListInstancesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_instance_admin::ListInstancesResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_LIST_INSTANCES, req, opt)
    }

    pub fn list_instances_async(&self, req: &super::bigtable_instance_admin::ListInstancesRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_instance_admin::ListInstancesResponse>> {
        self.list_instances_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_instance_opt(&self, req: &super::instance::Instance, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::instance::Instance> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_UPDATE_INSTANCE, req, opt)
    }

    pub fn update_instance(&self, req: &super::instance::Instance) -> ::grpcio::Result<super::instance::Instance> {
        self.update_instance_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_instance_async_opt(&self, req: &super::instance::Instance, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::instance::Instance>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_UPDATE_INSTANCE, req, opt)
    }

    pub fn update_instance_async(&self, req: &super::instance::Instance) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::instance::Instance>> {
        self.update_instance_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn partial_update_instance_opt(&self, req: &super::bigtable_instance_admin::PartialUpdateInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_PARTIAL_UPDATE_INSTANCE, req, opt)
    }

    pub fn partial_update_instance(&self, req: &super::bigtable_instance_admin::PartialUpdateInstanceRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.partial_update_instance_opt(req, ::grpcio::CallOption::default())
    }

    pub fn partial_update_instance_async_opt(&self, req: &super::bigtable_instance_admin::PartialUpdateInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_PARTIAL_UPDATE_INSTANCE, req, opt)
    }

    pub fn partial_update_instance_async(&self, req: &super::bigtable_instance_admin::PartialUpdateInstanceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.partial_update_instance_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_instance_opt(&self, req: &super::bigtable_instance_admin::DeleteInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_DELETE_INSTANCE, req, opt)
    }

    pub fn delete_instance(&self, req: &super::bigtable_instance_admin::DeleteInstanceRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_instance_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_instance_async_opt(&self, req: &super::bigtable_instance_admin::DeleteInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_DELETE_INSTANCE, req, opt)
    }

    pub fn delete_instance_async(&self, req: &super::bigtable_instance_admin::DeleteInstanceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_instance_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_cluster_opt(&self, req: &super::bigtable_instance_admin::CreateClusterRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_CREATE_CLUSTER, req, opt)
    }

    pub fn create_cluster(&self, req: &super::bigtable_instance_admin::CreateClusterRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.create_cluster_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_cluster_async_opt(&self, req: &super::bigtable_instance_admin::CreateClusterRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_CREATE_CLUSTER, req, opt)
    }

    pub fn create_cluster_async(&self, req: &super::bigtable_instance_admin::CreateClusterRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.create_cluster_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_cluster_opt(&self, req: &super::bigtable_instance_admin::GetClusterRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::instance::Cluster> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_GET_CLUSTER, req, opt)
    }

    pub fn get_cluster(&self, req: &super::bigtable_instance_admin::GetClusterRequest) -> ::grpcio::Result<super::instance::Cluster> {
        self.get_cluster_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_cluster_async_opt(&self, req: &super::bigtable_instance_admin::GetClusterRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::instance::Cluster>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_GET_CLUSTER, req, opt)
    }

    pub fn get_cluster_async(&self, req: &super::bigtable_instance_admin::GetClusterRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::instance::Cluster>> {
        self.get_cluster_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_clusters_opt(&self, req: &super::bigtable_instance_admin::ListClustersRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_instance_admin::ListClustersResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_LIST_CLUSTERS, req, opt)
    }

    pub fn list_clusters(&self, req: &super::bigtable_instance_admin::ListClustersRequest) -> ::grpcio::Result<super::bigtable_instance_admin::ListClustersResponse> {
        self.list_clusters_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_clusters_async_opt(&self, req: &super::bigtable_instance_admin::ListClustersRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_instance_admin::ListClustersResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_LIST_CLUSTERS, req, opt)
    }

    pub fn list_clusters_async(&self, req: &super::bigtable_instance_admin::ListClustersRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_instance_admin::ListClustersResponse>> {
        self.list_clusters_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_cluster_opt(&self, req: &super::instance::Cluster, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_UPDATE_CLUSTER, req, opt)
    }

    pub fn update_cluster(&self, req: &super::instance::Cluster) -> ::grpcio::Result<super::operations::Operation> {
        self.update_cluster_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_cluster_async_opt(&self, req: &super::instance::Cluster, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_UPDATE_CLUSTER, req, opt)
    }

    pub fn update_cluster_async(&self, req: &super::instance::Cluster) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.update_cluster_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_cluster_opt(&self, req: &super::bigtable_instance_admin::DeleteClusterRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_DELETE_CLUSTER, req, opt)
    }

    pub fn delete_cluster(&self, req: &super::bigtable_instance_admin::DeleteClusterRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_cluster_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_cluster_async_opt(&self, req: &super::bigtable_instance_admin::DeleteClusterRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_DELETE_CLUSTER, req, opt)
    }

    pub fn delete_cluster_async(&self, req: &super::bigtable_instance_admin::DeleteClusterRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_cluster_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_app_profile_opt(&self, req: &super::bigtable_instance_admin::CreateAppProfileRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::instance::AppProfile> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_CREATE_APP_PROFILE, req, opt)
    }

    pub fn create_app_profile(&self, req: &super::bigtable_instance_admin::CreateAppProfileRequest) -> ::grpcio::Result<super::instance::AppProfile> {
        self.create_app_profile_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_app_profile_async_opt(&self, req: &super::bigtable_instance_admin::CreateAppProfileRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::instance::AppProfile>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_CREATE_APP_PROFILE, req, opt)
    }

    pub fn create_app_profile_async(&self, req: &super::bigtable_instance_admin::CreateAppProfileRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::instance::AppProfile>> {
        self.create_app_profile_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_app_profile_opt(&self, req: &super::bigtable_instance_admin::GetAppProfileRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::instance::AppProfile> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_GET_APP_PROFILE, req, opt)
    }

    pub fn get_app_profile(&self, req: &super::bigtable_instance_admin::GetAppProfileRequest) -> ::grpcio::Result<super::instance::AppProfile> {
        self.get_app_profile_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_app_profile_async_opt(&self, req: &super::bigtable_instance_admin::GetAppProfileRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::instance::AppProfile>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_GET_APP_PROFILE, req, opt)
    }

    pub fn get_app_profile_async(&self, req: &super::bigtable_instance_admin::GetAppProfileRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::instance::AppProfile>> {
        self.get_app_profile_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_app_profiles_opt(&self, req: &super::bigtable_instance_admin::ListAppProfilesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_instance_admin::ListAppProfilesResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_LIST_APP_PROFILES, req, opt)
    }

    pub fn list_app_profiles(&self, req: &super::bigtable_instance_admin::ListAppProfilesRequest) -> ::grpcio::Result<super::bigtable_instance_admin::ListAppProfilesResponse> {
        self.list_app_profiles_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_app_profiles_async_opt(&self, req: &super::bigtable_instance_admin::ListAppProfilesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_instance_admin::ListAppProfilesResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_LIST_APP_PROFILES, req, opt)
    }

    pub fn list_app_profiles_async(&self, req: &super::bigtable_instance_admin::ListAppProfilesRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_instance_admin::ListAppProfilesResponse>> {
        self.list_app_profiles_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_app_profile_opt(&self, req: &super::bigtable_instance_admin::UpdateAppProfileRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_UPDATE_APP_PROFILE, req, opt)
    }

    pub fn update_app_profile(&self, req: &super::bigtable_instance_admin::UpdateAppProfileRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.update_app_profile_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_app_profile_async_opt(&self, req: &super::bigtable_instance_admin::UpdateAppProfileRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_UPDATE_APP_PROFILE, req, opt)
    }

    pub fn update_app_profile_async(&self, req: &super::bigtable_instance_admin::UpdateAppProfileRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.update_app_profile_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_app_profile_opt(&self, req: &super::bigtable_instance_admin::DeleteAppProfileRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_DELETE_APP_PROFILE, req, opt)
    }

    pub fn delete_app_profile(&self, req: &super::bigtable_instance_admin::DeleteAppProfileRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_app_profile_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_app_profile_async_opt(&self, req: &super::bigtable_instance_admin::DeleteAppProfileRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_DELETE_APP_PROFILE, req, opt)
    }

    pub fn delete_app_profile_async(&self, req: &super::bigtable_instance_admin::DeleteAppProfileRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_app_profile_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_iam_policy_opt(&self, req: &super::iam_policy::GetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::policy::Policy> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_GET_IAM_POLICY, req, opt)
    }

    pub fn get_iam_policy(&self, req: &super::iam_policy::GetIamPolicyRequest) -> ::grpcio::Result<super::policy::Policy> {
        self.get_iam_policy_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_iam_policy_async_opt(&self, req: &super::iam_policy::GetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_GET_IAM_POLICY, req, opt)
    }

    pub fn get_iam_policy_async(&self, req: &super::iam_policy::GetIamPolicyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.get_iam_policy_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_iam_policy_opt(&self, req: &super::iam_policy::SetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::policy::Policy> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_SET_IAM_POLICY, req, opt)
    }

    pub fn set_iam_policy(&self, req: &super::iam_policy::SetIamPolicyRequest) -> ::grpcio::Result<super::policy::Policy> {
        self.set_iam_policy_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_iam_policy_async_opt(&self, req: &super::iam_policy::SetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_SET_IAM_POLICY, req, opt)
    }

    pub fn set_iam_policy_async(&self, req: &super::iam_policy::SetIamPolicyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.set_iam_policy_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn test_iam_permissions_opt(&self, req: &super::iam_policy::TestIamPermissionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::iam_policy::TestIamPermissionsResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_INSTANCE_ADMIN_TEST_IAM_PERMISSIONS, req, opt)
    }

    pub fn test_iam_permissions(&self, req: &super::iam_policy::TestIamPermissionsRequest) -> ::grpcio::Result<super::iam_policy::TestIamPermissionsResponse> {
        self.test_iam_permissions_opt(req, ::grpcio::CallOption::default())
    }

    pub fn test_iam_permissions_async_opt(&self, req: &super::iam_policy::TestIamPermissionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::iam_policy::TestIamPermissionsResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_INSTANCE_ADMIN_TEST_IAM_PERMISSIONS, req, opt)
    }

    pub fn test_iam_permissions_async(&self, req: &super::iam_policy::TestIamPermissionsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::iam_policy::TestIamPermissionsResponse>> {
        self.test_iam_permissions_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait BigtableInstanceAdmin {
    fn create_instance(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_instance_admin::CreateInstanceRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn get_instance(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_instance_admin::GetInstanceRequest, sink: ::grpcio::UnarySink<super::instance::Instance>);
    fn list_instances(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_instance_admin::ListInstancesRequest, sink: ::grpcio::UnarySink<super::bigtable_instance_admin::ListInstancesResponse>);
    fn update_instance(&mut self, ctx: ::grpcio::RpcContext, req: super::instance::Instance, sink: ::grpcio::UnarySink<super::instance::Instance>);
    fn partial_update_instance(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_instance_admin::PartialUpdateInstanceRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn delete_instance(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_instance_admin::DeleteInstanceRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn create_cluster(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_instance_admin::CreateClusterRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn get_cluster(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_instance_admin::GetClusterRequest, sink: ::grpcio::UnarySink<super::instance::Cluster>);
    fn list_clusters(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_instance_admin::ListClustersRequest, sink: ::grpcio::UnarySink<super::bigtable_instance_admin::ListClustersResponse>);
    fn update_cluster(&mut self, ctx: ::grpcio::RpcContext, req: super::instance::Cluster, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn delete_cluster(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_instance_admin::DeleteClusterRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn create_app_profile(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_instance_admin::CreateAppProfileRequest, sink: ::grpcio::UnarySink<super::instance::AppProfile>);
    fn get_app_profile(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_instance_admin::GetAppProfileRequest, sink: ::grpcio::UnarySink<super::instance::AppProfile>);
    fn list_app_profiles(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_instance_admin::ListAppProfilesRequest, sink: ::grpcio::UnarySink<super::bigtable_instance_admin::ListAppProfilesResponse>);
    fn update_app_profile(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_instance_admin::UpdateAppProfileRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn delete_app_profile(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_instance_admin::DeleteAppProfileRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn get_iam_policy(&mut self, ctx: ::grpcio::RpcContext, req: super::iam_policy::GetIamPolicyRequest, sink: ::grpcio::UnarySink<super::policy::Policy>);
    fn set_iam_policy(&mut self, ctx: ::grpcio::RpcContext, req: super::iam_policy::SetIamPolicyRequest, sink: ::grpcio::UnarySink<super::policy::Policy>);
    fn test_iam_permissions(&mut self, ctx: ::grpcio::RpcContext, req: super::iam_policy::TestIamPermissionsRequest, sink: ::grpcio::UnarySink<super::iam_policy::TestIamPermissionsResponse>);
}

pub fn create_bigtable_instance_admin<S: BigtableInstanceAdmin + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_CREATE_INSTANCE, move |ctx, req, resp| {
        instance.create_instance(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_GET_INSTANCE, move |ctx, req, resp| {
        instance.get_instance(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_LIST_INSTANCES, move |ctx, req, resp| {
        instance.list_instances(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_UPDATE_INSTANCE, move |ctx, req, resp| {
        instance.update_instance(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_PARTIAL_UPDATE_INSTANCE, move |ctx, req, resp| {
        instance.partial_update_instance(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_DELETE_INSTANCE, move |ctx, req, resp| {
        instance.delete_instance(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_CREATE_CLUSTER, move |ctx, req, resp| {
        instance.create_cluster(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_GET_CLUSTER, move |ctx, req, resp| {
        instance.get_cluster(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_LIST_CLUSTERS, move |ctx, req, resp| {
        instance.list_clusters(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_UPDATE_CLUSTER, move |ctx, req, resp| {
        instance.update_cluster(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_DELETE_CLUSTER, move |ctx, req, resp| {
        instance.delete_cluster(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_CREATE_APP_PROFILE, move |ctx, req, resp| {
        instance.create_app_profile(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_GET_APP_PROFILE, move |ctx, req, resp| {
        instance.get_app_profile(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_LIST_APP_PROFILES, move |ctx, req, resp| {
        instance.list_app_profiles(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_UPDATE_APP_PROFILE, move |ctx, req, resp| {
        instance.update_app_profile(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_DELETE_APP_PROFILE, move |ctx, req, resp| {
        instance.delete_app_profile(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_GET_IAM_POLICY, move |ctx, req, resp| {
        instance.get_iam_policy(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_SET_IAM_POLICY, move |ctx, req, resp| {
        instance.set_iam_policy(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_INSTANCE_ADMIN_TEST_IAM_PERMISSIONS, move |ctx, req, resp| {
        instance.test_iam_permissions(ctx, req, resp)
    });
    builder.build()
}
