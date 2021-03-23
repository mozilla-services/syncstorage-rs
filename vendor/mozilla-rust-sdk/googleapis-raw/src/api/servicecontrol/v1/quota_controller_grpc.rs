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

const METHOD_QUOTA_CONTROLLER_ALLOCATE_QUOTA: ::grpcio::Method<super::quota_controller::AllocateQuotaRequest, super::quota_controller::AllocateQuotaResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicecontrol.v1.QuotaController/AllocateQuota",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct QuotaControllerClient {
    client: ::grpcio::Client,
}

impl QuotaControllerClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        QuotaControllerClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn allocate_quota_opt(&self, req: &super::quota_controller::AllocateQuotaRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::quota_controller::AllocateQuotaResponse> {
        self.client.unary_call(&METHOD_QUOTA_CONTROLLER_ALLOCATE_QUOTA, req, opt)
    }

    pub fn allocate_quota(&self, req: &super::quota_controller::AllocateQuotaRequest) -> ::grpcio::Result<super::quota_controller::AllocateQuotaResponse> {
        self.allocate_quota_opt(req, ::grpcio::CallOption::default())
    }

    pub fn allocate_quota_async_opt(&self, req: &super::quota_controller::AllocateQuotaRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::quota_controller::AllocateQuotaResponse>> {
        self.client.unary_call_async(&METHOD_QUOTA_CONTROLLER_ALLOCATE_QUOTA, req, opt)
    }

    pub fn allocate_quota_async(&self, req: &super::quota_controller::AllocateQuotaRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::quota_controller::AllocateQuotaResponse>> {
        self.allocate_quota_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Output = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait QuotaController {
    fn allocate_quota(&mut self, ctx: ::grpcio::RpcContext, req: super::quota_controller::AllocateQuotaRequest, sink: ::grpcio::UnarySink<super::quota_controller::AllocateQuotaResponse>);
}

pub fn create_quota_controller<S: QuotaController + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s;
    builder = builder.add_unary_handler(&METHOD_QUOTA_CONTROLLER_ALLOCATE_QUOTA, move |ctx, req, resp| {
        instance.allocate_quota(ctx, req, resp)
    });
    builder.build()
}
