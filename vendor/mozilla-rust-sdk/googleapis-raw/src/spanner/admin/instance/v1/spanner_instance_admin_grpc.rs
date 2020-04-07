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

const METHOD_INSTANCE_ADMIN_LIST_INSTANCE_CONFIGS: ::grpcio::Method<super::spanner_instance_admin::ListInstanceConfigsRequest, super::spanner_instance_admin::ListInstanceConfigsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.instance.v1.InstanceAdmin/ListInstanceConfigs",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_INSTANCE_ADMIN_GET_INSTANCE_CONFIG: ::grpcio::Method<super::spanner_instance_admin::GetInstanceConfigRequest, super::spanner_instance_admin::InstanceConfig> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.instance.v1.InstanceAdmin/GetInstanceConfig",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_INSTANCE_ADMIN_LIST_INSTANCES: ::grpcio::Method<super::spanner_instance_admin::ListInstancesRequest, super::spanner_instance_admin::ListInstancesResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.instance.v1.InstanceAdmin/ListInstances",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_INSTANCE_ADMIN_GET_INSTANCE: ::grpcio::Method<super::spanner_instance_admin::GetInstanceRequest, super::spanner_instance_admin::Instance> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.instance.v1.InstanceAdmin/GetInstance",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_INSTANCE_ADMIN_CREATE_INSTANCE: ::grpcio::Method<super::spanner_instance_admin::CreateInstanceRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.instance.v1.InstanceAdmin/CreateInstance",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_INSTANCE_ADMIN_UPDATE_INSTANCE: ::grpcio::Method<super::spanner_instance_admin::UpdateInstanceRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.instance.v1.InstanceAdmin/UpdateInstance",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_INSTANCE_ADMIN_DELETE_INSTANCE: ::grpcio::Method<super::spanner_instance_admin::DeleteInstanceRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.instance.v1.InstanceAdmin/DeleteInstance",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_INSTANCE_ADMIN_SET_IAM_POLICY: ::grpcio::Method<super::iam_policy::SetIamPolicyRequest, super::policy::Policy> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.instance.v1.InstanceAdmin/SetIamPolicy",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_INSTANCE_ADMIN_GET_IAM_POLICY: ::grpcio::Method<super::iam_policy::GetIamPolicyRequest, super::policy::Policy> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.instance.v1.InstanceAdmin/GetIamPolicy",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_INSTANCE_ADMIN_TEST_IAM_PERMISSIONS: ::grpcio::Method<super::iam_policy::TestIamPermissionsRequest, super::iam_policy::TestIamPermissionsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.instance.v1.InstanceAdmin/TestIamPermissions",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct InstanceAdminClient {
    client: ::grpcio::Client,
}

impl InstanceAdminClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        InstanceAdminClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn list_instance_configs_opt(&self, req: &super::spanner_instance_admin::ListInstanceConfigsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::spanner_instance_admin::ListInstanceConfigsResponse> {
        self.client.unary_call(&METHOD_INSTANCE_ADMIN_LIST_INSTANCE_CONFIGS, req, opt)
    }

    pub fn list_instance_configs(&self, req: &super::spanner_instance_admin::ListInstanceConfigsRequest) -> ::grpcio::Result<super::spanner_instance_admin::ListInstanceConfigsResponse> {
        self.list_instance_configs_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_instance_configs_async_opt(&self, req: &super::spanner_instance_admin::ListInstanceConfigsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_instance_admin::ListInstanceConfigsResponse>> {
        self.client.unary_call_async(&METHOD_INSTANCE_ADMIN_LIST_INSTANCE_CONFIGS, req, opt)
    }

    pub fn list_instance_configs_async(&self, req: &super::spanner_instance_admin::ListInstanceConfigsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_instance_admin::ListInstanceConfigsResponse>> {
        self.list_instance_configs_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_instance_config_opt(&self, req: &super::spanner_instance_admin::GetInstanceConfigRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::spanner_instance_admin::InstanceConfig> {
        self.client.unary_call(&METHOD_INSTANCE_ADMIN_GET_INSTANCE_CONFIG, req, opt)
    }

    pub fn get_instance_config(&self, req: &super::spanner_instance_admin::GetInstanceConfigRequest) -> ::grpcio::Result<super::spanner_instance_admin::InstanceConfig> {
        self.get_instance_config_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_instance_config_async_opt(&self, req: &super::spanner_instance_admin::GetInstanceConfigRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_instance_admin::InstanceConfig>> {
        self.client.unary_call_async(&METHOD_INSTANCE_ADMIN_GET_INSTANCE_CONFIG, req, opt)
    }

    pub fn get_instance_config_async(&self, req: &super::spanner_instance_admin::GetInstanceConfigRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_instance_admin::InstanceConfig>> {
        self.get_instance_config_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_instances_opt(&self, req: &super::spanner_instance_admin::ListInstancesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::spanner_instance_admin::ListInstancesResponse> {
        self.client.unary_call(&METHOD_INSTANCE_ADMIN_LIST_INSTANCES, req, opt)
    }

    pub fn list_instances(&self, req: &super::spanner_instance_admin::ListInstancesRequest) -> ::grpcio::Result<super::spanner_instance_admin::ListInstancesResponse> {
        self.list_instances_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_instances_async_opt(&self, req: &super::spanner_instance_admin::ListInstancesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_instance_admin::ListInstancesResponse>> {
        self.client.unary_call_async(&METHOD_INSTANCE_ADMIN_LIST_INSTANCES, req, opt)
    }

    pub fn list_instances_async(&self, req: &super::spanner_instance_admin::ListInstancesRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_instance_admin::ListInstancesResponse>> {
        self.list_instances_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_instance_opt(&self, req: &super::spanner_instance_admin::GetInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::spanner_instance_admin::Instance> {
        self.client.unary_call(&METHOD_INSTANCE_ADMIN_GET_INSTANCE, req, opt)
    }

    pub fn get_instance(&self, req: &super::spanner_instance_admin::GetInstanceRequest) -> ::grpcio::Result<super::spanner_instance_admin::Instance> {
        self.get_instance_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_instance_async_opt(&self, req: &super::spanner_instance_admin::GetInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_instance_admin::Instance>> {
        self.client.unary_call_async(&METHOD_INSTANCE_ADMIN_GET_INSTANCE, req, opt)
    }

    pub fn get_instance_async(&self, req: &super::spanner_instance_admin::GetInstanceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_instance_admin::Instance>> {
        self.get_instance_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_instance_opt(&self, req: &super::spanner_instance_admin::CreateInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_INSTANCE_ADMIN_CREATE_INSTANCE, req, opt)
    }

    pub fn create_instance(&self, req: &super::spanner_instance_admin::CreateInstanceRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.create_instance_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_instance_async_opt(&self, req: &super::spanner_instance_admin::CreateInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_INSTANCE_ADMIN_CREATE_INSTANCE, req, opt)
    }

    pub fn create_instance_async(&self, req: &super::spanner_instance_admin::CreateInstanceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.create_instance_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_instance_opt(&self, req: &super::spanner_instance_admin::UpdateInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_INSTANCE_ADMIN_UPDATE_INSTANCE, req, opt)
    }

    pub fn update_instance(&self, req: &super::spanner_instance_admin::UpdateInstanceRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.update_instance_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_instance_async_opt(&self, req: &super::spanner_instance_admin::UpdateInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_INSTANCE_ADMIN_UPDATE_INSTANCE, req, opt)
    }

    pub fn update_instance_async(&self, req: &super::spanner_instance_admin::UpdateInstanceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.update_instance_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_instance_opt(&self, req: &super::spanner_instance_admin::DeleteInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_INSTANCE_ADMIN_DELETE_INSTANCE, req, opt)
    }

    pub fn delete_instance(&self, req: &super::spanner_instance_admin::DeleteInstanceRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_instance_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_instance_async_opt(&self, req: &super::spanner_instance_admin::DeleteInstanceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_INSTANCE_ADMIN_DELETE_INSTANCE, req, opt)
    }

    pub fn delete_instance_async(&self, req: &super::spanner_instance_admin::DeleteInstanceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_instance_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_iam_policy_opt(&self, req: &super::iam_policy::SetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::policy::Policy> {
        self.client.unary_call(&METHOD_INSTANCE_ADMIN_SET_IAM_POLICY, req, opt)
    }

    pub fn set_iam_policy(&self, req: &super::iam_policy::SetIamPolicyRequest) -> ::grpcio::Result<super::policy::Policy> {
        self.set_iam_policy_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_iam_policy_async_opt(&self, req: &super::iam_policy::SetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.client.unary_call_async(&METHOD_INSTANCE_ADMIN_SET_IAM_POLICY, req, opt)
    }

    pub fn set_iam_policy_async(&self, req: &super::iam_policy::SetIamPolicyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.set_iam_policy_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_iam_policy_opt(&self, req: &super::iam_policy::GetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::policy::Policy> {
        self.client.unary_call(&METHOD_INSTANCE_ADMIN_GET_IAM_POLICY, req, opt)
    }

    pub fn get_iam_policy(&self, req: &super::iam_policy::GetIamPolicyRequest) -> ::grpcio::Result<super::policy::Policy> {
        self.get_iam_policy_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_iam_policy_async_opt(&self, req: &super::iam_policy::GetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.client.unary_call_async(&METHOD_INSTANCE_ADMIN_GET_IAM_POLICY, req, opt)
    }

    pub fn get_iam_policy_async(&self, req: &super::iam_policy::GetIamPolicyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.get_iam_policy_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn test_iam_permissions_opt(&self, req: &super::iam_policy::TestIamPermissionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::iam_policy::TestIamPermissionsResponse> {
        self.client.unary_call(&METHOD_INSTANCE_ADMIN_TEST_IAM_PERMISSIONS, req, opt)
    }

    pub fn test_iam_permissions(&self, req: &super::iam_policy::TestIamPermissionsRequest) -> ::grpcio::Result<super::iam_policy::TestIamPermissionsResponse> {
        self.test_iam_permissions_opt(req, ::grpcio::CallOption::default())
    }

    pub fn test_iam_permissions_async_opt(&self, req: &super::iam_policy::TestIamPermissionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::iam_policy::TestIamPermissionsResponse>> {
        self.client.unary_call_async(&METHOD_INSTANCE_ADMIN_TEST_IAM_PERMISSIONS, req, opt)
    }

    pub fn test_iam_permissions_async(&self, req: &super::iam_policy::TestIamPermissionsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::iam_policy::TestIamPermissionsResponse>> {
        self.test_iam_permissions_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait InstanceAdmin {
    fn list_instance_configs(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_instance_admin::ListInstanceConfigsRequest, sink: ::grpcio::UnarySink<super::spanner_instance_admin::ListInstanceConfigsResponse>);
    fn get_instance_config(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_instance_admin::GetInstanceConfigRequest, sink: ::grpcio::UnarySink<super::spanner_instance_admin::InstanceConfig>);
    fn list_instances(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_instance_admin::ListInstancesRequest, sink: ::grpcio::UnarySink<super::spanner_instance_admin::ListInstancesResponse>);
    fn get_instance(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_instance_admin::GetInstanceRequest, sink: ::grpcio::UnarySink<super::spanner_instance_admin::Instance>);
    fn create_instance(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_instance_admin::CreateInstanceRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn update_instance(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_instance_admin::UpdateInstanceRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn delete_instance(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_instance_admin::DeleteInstanceRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn set_iam_policy(&mut self, ctx: ::grpcio::RpcContext, req: super::iam_policy::SetIamPolicyRequest, sink: ::grpcio::UnarySink<super::policy::Policy>);
    fn get_iam_policy(&mut self, ctx: ::grpcio::RpcContext, req: super::iam_policy::GetIamPolicyRequest, sink: ::grpcio::UnarySink<super::policy::Policy>);
    fn test_iam_permissions(&mut self, ctx: ::grpcio::RpcContext, req: super::iam_policy::TestIamPermissionsRequest, sink: ::grpcio::UnarySink<super::iam_policy::TestIamPermissionsResponse>);
}

pub fn create_instance_admin<S: InstanceAdmin + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_INSTANCE_ADMIN_LIST_INSTANCE_CONFIGS, move |ctx, req, resp| {
        instance.list_instance_configs(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_INSTANCE_ADMIN_GET_INSTANCE_CONFIG, move |ctx, req, resp| {
        instance.get_instance_config(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_INSTANCE_ADMIN_LIST_INSTANCES, move |ctx, req, resp| {
        instance.list_instances(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_INSTANCE_ADMIN_GET_INSTANCE, move |ctx, req, resp| {
        instance.get_instance(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_INSTANCE_ADMIN_CREATE_INSTANCE, move |ctx, req, resp| {
        instance.create_instance(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_INSTANCE_ADMIN_UPDATE_INSTANCE, move |ctx, req, resp| {
        instance.update_instance(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_INSTANCE_ADMIN_DELETE_INSTANCE, move |ctx, req, resp| {
        instance.delete_instance(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_INSTANCE_ADMIN_SET_IAM_POLICY, move |ctx, req, resp| {
        instance.set_iam_policy(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_INSTANCE_ADMIN_GET_IAM_POLICY, move |ctx, req, resp| {
        instance.get_iam_policy(ctx, req, resp)
    });
    let mut instance = s;
    builder = builder.add_unary_handler(&METHOD_INSTANCE_ADMIN_TEST_IAM_PERMISSIONS, move |ctx, req, resp| {
        instance.test_iam_permissions(ctx, req, resp)
    });
    builder.build()
}
