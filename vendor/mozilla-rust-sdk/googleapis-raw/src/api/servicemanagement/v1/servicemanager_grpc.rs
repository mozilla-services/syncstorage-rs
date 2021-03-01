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

const METHOD_SERVICE_MANAGER_LIST_SERVICES: ::grpcio::Method<super::servicemanager::ListServicesRequest, super::servicemanager::ListServicesResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/ListServices",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_MANAGER_GET_SERVICE: ::grpcio::Method<super::servicemanager::GetServiceRequest, super::resources::ManagedService> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/GetService",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_MANAGER_CREATE_SERVICE: ::grpcio::Method<super::servicemanager::CreateServiceRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/CreateService",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_MANAGER_DELETE_SERVICE: ::grpcio::Method<super::servicemanager::DeleteServiceRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/DeleteService",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_MANAGER_UNDELETE_SERVICE: ::grpcio::Method<super::servicemanager::UndeleteServiceRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/UndeleteService",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_MANAGER_LIST_SERVICE_CONFIGS: ::grpcio::Method<super::servicemanager::ListServiceConfigsRequest, super::servicemanager::ListServiceConfigsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/ListServiceConfigs",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_MANAGER_GET_SERVICE_CONFIG: ::grpcio::Method<super::servicemanager::GetServiceConfigRequest, super::service::Service> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/GetServiceConfig",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_MANAGER_CREATE_SERVICE_CONFIG: ::grpcio::Method<super::servicemanager::CreateServiceConfigRequest, super::service::Service> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/CreateServiceConfig",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_MANAGER_SUBMIT_CONFIG_SOURCE: ::grpcio::Method<super::servicemanager::SubmitConfigSourceRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/SubmitConfigSource",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_MANAGER_LIST_SERVICE_ROLLOUTS: ::grpcio::Method<super::servicemanager::ListServiceRolloutsRequest, super::servicemanager::ListServiceRolloutsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/ListServiceRollouts",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_MANAGER_GET_SERVICE_ROLLOUT: ::grpcio::Method<super::servicemanager::GetServiceRolloutRequest, super::resources::Rollout> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/GetServiceRollout",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_MANAGER_CREATE_SERVICE_ROLLOUT: ::grpcio::Method<super::servicemanager::CreateServiceRolloutRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/CreateServiceRollout",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_MANAGER_GENERATE_CONFIG_REPORT: ::grpcio::Method<super::servicemanager::GenerateConfigReportRequest, super::servicemanager::GenerateConfigReportResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/GenerateConfigReport",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_MANAGER_ENABLE_SERVICE: ::grpcio::Method<super::servicemanager::EnableServiceRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/EnableService",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_MANAGER_DISABLE_SERVICE: ::grpcio::Method<super::servicemanager::DisableServiceRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicemanagement.v1.ServiceManager/DisableService",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct ServiceManagerClient {
    client: ::grpcio::Client,
}

impl ServiceManagerClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        ServiceManagerClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn list_services_opt(&self, req: &super::servicemanager::ListServicesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::servicemanager::ListServicesResponse> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_LIST_SERVICES, req, opt)
    }

    pub fn list_services(&self, req: &super::servicemanager::ListServicesRequest) -> ::grpcio::Result<super::servicemanager::ListServicesResponse> {
        self.list_services_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_services_async_opt(&self, req: &super::servicemanager::ListServicesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::servicemanager::ListServicesResponse>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_LIST_SERVICES, req, opt)
    }

    pub fn list_services_async(&self, req: &super::servicemanager::ListServicesRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::servicemanager::ListServicesResponse>> {
        self.list_services_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_service_opt(&self, req: &super::servicemanager::GetServiceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::resources::ManagedService> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_GET_SERVICE, req, opt)
    }

    pub fn get_service(&self, req: &super::servicemanager::GetServiceRequest) -> ::grpcio::Result<super::resources::ManagedService> {
        self.get_service_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_service_async_opt(&self, req: &super::servicemanager::GetServiceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::resources::ManagedService>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_GET_SERVICE, req, opt)
    }

    pub fn get_service_async(&self, req: &super::servicemanager::GetServiceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::resources::ManagedService>> {
        self.get_service_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_service_opt(&self, req: &super::servicemanager::CreateServiceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_CREATE_SERVICE, req, opt)
    }

    pub fn create_service(&self, req: &super::servicemanager::CreateServiceRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.create_service_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_service_async_opt(&self, req: &super::servicemanager::CreateServiceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_CREATE_SERVICE, req, opt)
    }

    pub fn create_service_async(&self, req: &super::servicemanager::CreateServiceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.create_service_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_service_opt(&self, req: &super::servicemanager::DeleteServiceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_DELETE_SERVICE, req, opt)
    }

    pub fn delete_service(&self, req: &super::servicemanager::DeleteServiceRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.delete_service_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_service_async_opt(&self, req: &super::servicemanager::DeleteServiceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_DELETE_SERVICE, req, opt)
    }

    pub fn delete_service_async(&self, req: &super::servicemanager::DeleteServiceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.delete_service_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn undelete_service_opt(&self, req: &super::servicemanager::UndeleteServiceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_UNDELETE_SERVICE, req, opt)
    }

    pub fn undelete_service(&self, req: &super::servicemanager::UndeleteServiceRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.undelete_service_opt(req, ::grpcio::CallOption::default())
    }

    pub fn undelete_service_async_opt(&self, req: &super::servicemanager::UndeleteServiceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_UNDELETE_SERVICE, req, opt)
    }

    pub fn undelete_service_async(&self, req: &super::servicemanager::UndeleteServiceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.undelete_service_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_service_configs_opt(&self, req: &super::servicemanager::ListServiceConfigsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::servicemanager::ListServiceConfigsResponse> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_LIST_SERVICE_CONFIGS, req, opt)
    }

    pub fn list_service_configs(&self, req: &super::servicemanager::ListServiceConfigsRequest) -> ::grpcio::Result<super::servicemanager::ListServiceConfigsResponse> {
        self.list_service_configs_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_service_configs_async_opt(&self, req: &super::servicemanager::ListServiceConfigsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::servicemanager::ListServiceConfigsResponse>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_LIST_SERVICE_CONFIGS, req, opt)
    }

    pub fn list_service_configs_async(&self, req: &super::servicemanager::ListServiceConfigsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::servicemanager::ListServiceConfigsResponse>> {
        self.list_service_configs_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_service_config_opt(&self, req: &super::servicemanager::GetServiceConfigRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::service::Service> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_GET_SERVICE_CONFIG, req, opt)
    }

    pub fn get_service_config(&self, req: &super::servicemanager::GetServiceConfigRequest) -> ::grpcio::Result<super::service::Service> {
        self.get_service_config_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_service_config_async_opt(&self, req: &super::servicemanager::GetServiceConfigRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::service::Service>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_GET_SERVICE_CONFIG, req, opt)
    }

    pub fn get_service_config_async(&self, req: &super::servicemanager::GetServiceConfigRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::service::Service>> {
        self.get_service_config_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_service_config_opt(&self, req: &super::servicemanager::CreateServiceConfigRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::service::Service> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_CREATE_SERVICE_CONFIG, req, opt)
    }

    pub fn create_service_config(&self, req: &super::servicemanager::CreateServiceConfigRequest) -> ::grpcio::Result<super::service::Service> {
        self.create_service_config_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_service_config_async_opt(&self, req: &super::servicemanager::CreateServiceConfigRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::service::Service>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_CREATE_SERVICE_CONFIG, req, opt)
    }

    pub fn create_service_config_async(&self, req: &super::servicemanager::CreateServiceConfigRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::service::Service>> {
        self.create_service_config_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn submit_config_source_opt(&self, req: &super::servicemanager::SubmitConfigSourceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_SUBMIT_CONFIG_SOURCE, req, opt)
    }

    pub fn submit_config_source(&self, req: &super::servicemanager::SubmitConfigSourceRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.submit_config_source_opt(req, ::grpcio::CallOption::default())
    }

    pub fn submit_config_source_async_opt(&self, req: &super::servicemanager::SubmitConfigSourceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_SUBMIT_CONFIG_SOURCE, req, opt)
    }

    pub fn submit_config_source_async(&self, req: &super::servicemanager::SubmitConfigSourceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.submit_config_source_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_service_rollouts_opt(&self, req: &super::servicemanager::ListServiceRolloutsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::servicemanager::ListServiceRolloutsResponse> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_LIST_SERVICE_ROLLOUTS, req, opt)
    }

    pub fn list_service_rollouts(&self, req: &super::servicemanager::ListServiceRolloutsRequest) -> ::grpcio::Result<super::servicemanager::ListServiceRolloutsResponse> {
        self.list_service_rollouts_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_service_rollouts_async_opt(&self, req: &super::servicemanager::ListServiceRolloutsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::servicemanager::ListServiceRolloutsResponse>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_LIST_SERVICE_ROLLOUTS, req, opt)
    }

    pub fn list_service_rollouts_async(&self, req: &super::servicemanager::ListServiceRolloutsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::servicemanager::ListServiceRolloutsResponse>> {
        self.list_service_rollouts_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_service_rollout_opt(&self, req: &super::servicemanager::GetServiceRolloutRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::resources::Rollout> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_GET_SERVICE_ROLLOUT, req, opt)
    }

    pub fn get_service_rollout(&self, req: &super::servicemanager::GetServiceRolloutRequest) -> ::grpcio::Result<super::resources::Rollout> {
        self.get_service_rollout_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_service_rollout_async_opt(&self, req: &super::servicemanager::GetServiceRolloutRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::resources::Rollout>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_GET_SERVICE_ROLLOUT, req, opt)
    }

    pub fn get_service_rollout_async(&self, req: &super::servicemanager::GetServiceRolloutRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::resources::Rollout>> {
        self.get_service_rollout_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_service_rollout_opt(&self, req: &super::servicemanager::CreateServiceRolloutRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_CREATE_SERVICE_ROLLOUT, req, opt)
    }

    pub fn create_service_rollout(&self, req: &super::servicemanager::CreateServiceRolloutRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.create_service_rollout_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_service_rollout_async_opt(&self, req: &super::servicemanager::CreateServiceRolloutRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_CREATE_SERVICE_ROLLOUT, req, opt)
    }

    pub fn create_service_rollout_async(&self, req: &super::servicemanager::CreateServiceRolloutRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.create_service_rollout_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn generate_config_report_opt(&self, req: &super::servicemanager::GenerateConfigReportRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::servicemanager::GenerateConfigReportResponse> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_GENERATE_CONFIG_REPORT, req, opt)
    }

    pub fn generate_config_report(&self, req: &super::servicemanager::GenerateConfigReportRequest) -> ::grpcio::Result<super::servicemanager::GenerateConfigReportResponse> {
        self.generate_config_report_opt(req, ::grpcio::CallOption::default())
    }

    pub fn generate_config_report_async_opt(&self, req: &super::servicemanager::GenerateConfigReportRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::servicemanager::GenerateConfigReportResponse>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_GENERATE_CONFIG_REPORT, req, opt)
    }

    pub fn generate_config_report_async(&self, req: &super::servicemanager::GenerateConfigReportRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::servicemanager::GenerateConfigReportResponse>> {
        self.generate_config_report_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn enable_service_opt(&self, req: &super::servicemanager::EnableServiceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_ENABLE_SERVICE, req, opt)
    }

    pub fn enable_service(&self, req: &super::servicemanager::EnableServiceRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.enable_service_opt(req, ::grpcio::CallOption::default())
    }

    pub fn enable_service_async_opt(&self, req: &super::servicemanager::EnableServiceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_ENABLE_SERVICE, req, opt)
    }

    pub fn enable_service_async(&self, req: &super::servicemanager::EnableServiceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.enable_service_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn disable_service_opt(&self, req: &super::servicemanager::DisableServiceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_SERVICE_MANAGER_DISABLE_SERVICE, req, opt)
    }

    pub fn disable_service(&self, req: &super::servicemanager::DisableServiceRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.disable_service_opt(req, ::grpcio::CallOption::default())
    }

    pub fn disable_service_async_opt(&self, req: &super::servicemanager::DisableServiceRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_SERVICE_MANAGER_DISABLE_SERVICE, req, opt)
    }

    pub fn disable_service_async(&self, req: &super::servicemanager::DisableServiceRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.disable_service_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Output = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait ServiceManager {
    fn list_services(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::ListServicesRequest, sink: ::grpcio::UnarySink<super::servicemanager::ListServicesResponse>);
    fn get_service(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::GetServiceRequest, sink: ::grpcio::UnarySink<super::resources::ManagedService>);
    fn create_service(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::CreateServiceRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn delete_service(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::DeleteServiceRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn undelete_service(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::UndeleteServiceRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn list_service_configs(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::ListServiceConfigsRequest, sink: ::grpcio::UnarySink<super::servicemanager::ListServiceConfigsResponse>);
    fn get_service_config(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::GetServiceConfigRequest, sink: ::grpcio::UnarySink<super::service::Service>);
    fn create_service_config(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::CreateServiceConfigRequest, sink: ::grpcio::UnarySink<super::service::Service>);
    fn submit_config_source(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::SubmitConfigSourceRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn list_service_rollouts(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::ListServiceRolloutsRequest, sink: ::grpcio::UnarySink<super::servicemanager::ListServiceRolloutsResponse>);
    fn get_service_rollout(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::GetServiceRolloutRequest, sink: ::grpcio::UnarySink<super::resources::Rollout>);
    fn create_service_rollout(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::CreateServiceRolloutRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn generate_config_report(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::GenerateConfigReportRequest, sink: ::grpcio::UnarySink<super::servicemanager::GenerateConfigReportResponse>);
    fn enable_service(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::EnableServiceRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn disable_service(&mut self, ctx: ::grpcio::RpcContext, req: super::servicemanager::DisableServiceRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
}

pub fn create_service_manager<S: ServiceManager + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_LIST_SERVICES, move |ctx, req, resp| {
        instance.list_services(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_GET_SERVICE, move |ctx, req, resp| {
        instance.get_service(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_CREATE_SERVICE, move |ctx, req, resp| {
        instance.create_service(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_DELETE_SERVICE, move |ctx, req, resp| {
        instance.delete_service(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_UNDELETE_SERVICE, move |ctx, req, resp| {
        instance.undelete_service(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_LIST_SERVICE_CONFIGS, move |ctx, req, resp| {
        instance.list_service_configs(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_GET_SERVICE_CONFIG, move |ctx, req, resp| {
        instance.get_service_config(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_CREATE_SERVICE_CONFIG, move |ctx, req, resp| {
        instance.create_service_config(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_SUBMIT_CONFIG_SOURCE, move |ctx, req, resp| {
        instance.submit_config_source(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_LIST_SERVICE_ROLLOUTS, move |ctx, req, resp| {
        instance.list_service_rollouts(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_GET_SERVICE_ROLLOUT, move |ctx, req, resp| {
        instance.get_service_rollout(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_CREATE_SERVICE_ROLLOUT, move |ctx, req, resp| {
        instance.create_service_rollout(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_GENERATE_CONFIG_REPORT, move |ctx, req, resp| {
        instance.generate_config_report(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_ENABLE_SERVICE, move |ctx, req, resp| {
        instance.enable_service(ctx, req, resp)
    });
    let mut instance = s;
    builder = builder.add_unary_handler(&METHOD_SERVICE_MANAGER_DISABLE_SERVICE, move |ctx, req, resp| {
        instance.disable_service(ctx, req, resp)
    });
    builder.build()
}
