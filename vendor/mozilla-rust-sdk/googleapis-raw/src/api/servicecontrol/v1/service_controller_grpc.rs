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

const METHOD_SERVICE_CONTROLLER_CHECK: ::grpcio::Method<super::service_controller::CheckRequest, super::service_controller::CheckResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicecontrol.v1.ServiceController/Check",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SERVICE_CONTROLLER_REPORT: ::grpcio::Method<super::service_controller::ReportRequest, super::service_controller::ReportResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.api.servicecontrol.v1.ServiceController/Report",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct ServiceControllerClient {
    client: ::grpcio::Client,
}

impl ServiceControllerClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        ServiceControllerClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn check_opt(&self, req: &super::service_controller::CheckRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::service_controller::CheckResponse> {
        self.client.unary_call(&METHOD_SERVICE_CONTROLLER_CHECK, req, opt)
    }

    pub fn check(&self, req: &super::service_controller::CheckRequest) -> ::grpcio::Result<super::service_controller::CheckResponse> {
        self.check_opt(req, ::grpcio::CallOption::default())
    }

    pub fn check_async_opt(&self, req: &super::service_controller::CheckRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::service_controller::CheckResponse>> {
        self.client.unary_call_async(&METHOD_SERVICE_CONTROLLER_CHECK, req, opt)
    }

    pub fn check_async(&self, req: &super::service_controller::CheckRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::service_controller::CheckResponse>> {
        self.check_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn report_opt(&self, req: &super::service_controller::ReportRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::service_controller::ReportResponse> {
        self.client.unary_call(&METHOD_SERVICE_CONTROLLER_REPORT, req, opt)
    }

    pub fn report(&self, req: &super::service_controller::ReportRequest) -> ::grpcio::Result<super::service_controller::ReportResponse> {
        self.report_opt(req, ::grpcio::CallOption::default())
    }

    pub fn report_async_opt(&self, req: &super::service_controller::ReportRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::service_controller::ReportResponse>> {
        self.client.unary_call_async(&METHOD_SERVICE_CONTROLLER_REPORT, req, opt)
    }

    pub fn report_async(&self, req: &super::service_controller::ReportRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::service_controller::ReportResponse>> {
        self.report_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Output = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait ServiceController {
    fn check(&mut self, ctx: ::grpcio::RpcContext, req: super::service_controller::CheckRequest, sink: ::grpcio::UnarySink<super::service_controller::CheckResponse>);
    fn report(&mut self, ctx: ::grpcio::RpcContext, req: super::service_controller::ReportRequest, sink: ::grpcio::UnarySink<super::service_controller::ReportResponse>);
}

pub fn create_service_controller<S: ServiceController + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SERVICE_CONTROLLER_CHECK, move |ctx, req, resp| {
        instance.check(ctx, req, resp)
    });
    let mut instance = s;
    builder = builder.add_unary_handler(&METHOD_SERVICE_CONTROLLER_REPORT, move |ctx, req, resp| {
        instance.report(ctx, req, resp)
    });
    builder.build()
}
