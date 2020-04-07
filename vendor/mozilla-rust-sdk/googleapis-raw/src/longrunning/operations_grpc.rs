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

const METHOD_OPERATIONS_LIST_OPERATIONS: ::grpcio::Method<super::operations::ListOperationsRequest, super::operations::ListOperationsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.longrunning.Operations/ListOperations",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_OPERATIONS_GET_OPERATION: ::grpcio::Method<super::operations::GetOperationRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.longrunning.Operations/GetOperation",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_OPERATIONS_DELETE_OPERATION: ::grpcio::Method<super::operations::DeleteOperationRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.longrunning.Operations/DeleteOperation",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_OPERATIONS_CANCEL_OPERATION: ::grpcio::Method<super::operations::CancelOperationRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.longrunning.Operations/CancelOperation",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct OperationsClient {
    client: ::grpcio::Client,
}

impl OperationsClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        OperationsClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn list_operations_opt(&self, req: &super::operations::ListOperationsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::ListOperationsResponse> {
        self.client.unary_call(&METHOD_OPERATIONS_LIST_OPERATIONS, req, opt)
    }

    pub fn list_operations(&self, req: &super::operations::ListOperationsRequest) -> ::grpcio::Result<super::operations::ListOperationsResponse> {
        self.list_operations_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_operations_async_opt(&self, req: &super::operations::ListOperationsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::ListOperationsResponse>> {
        self.client.unary_call_async(&METHOD_OPERATIONS_LIST_OPERATIONS, req, opt)
    }

    pub fn list_operations_async(&self, req: &super::operations::ListOperationsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::ListOperationsResponse>> {
        self.list_operations_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_operation_opt(&self, req: &super::operations::GetOperationRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_OPERATIONS_GET_OPERATION, req, opt)
    }

    pub fn get_operation(&self, req: &super::operations::GetOperationRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.get_operation_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_operation_async_opt(&self, req: &super::operations::GetOperationRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_OPERATIONS_GET_OPERATION, req, opt)
    }

    pub fn get_operation_async(&self, req: &super::operations::GetOperationRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.get_operation_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_operation_opt(&self, req: &super::operations::DeleteOperationRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_OPERATIONS_DELETE_OPERATION, req, opt)
    }

    pub fn delete_operation(&self, req: &super::operations::DeleteOperationRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_operation_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_operation_async_opt(&self, req: &super::operations::DeleteOperationRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_OPERATIONS_DELETE_OPERATION, req, opt)
    }

    pub fn delete_operation_async(&self, req: &super::operations::DeleteOperationRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_operation_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn cancel_operation_opt(&self, req: &super::operations::CancelOperationRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_OPERATIONS_CANCEL_OPERATION, req, opt)
    }

    pub fn cancel_operation(&self, req: &super::operations::CancelOperationRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.cancel_operation_opt(req, ::grpcio::CallOption::default())
    }

    pub fn cancel_operation_async_opt(&self, req: &super::operations::CancelOperationRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_OPERATIONS_CANCEL_OPERATION, req, opt)
    }

    pub fn cancel_operation_async(&self, req: &super::operations::CancelOperationRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.cancel_operation_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait Operations {
    fn list_operations(&mut self, ctx: ::grpcio::RpcContext, req: super::operations::ListOperationsRequest, sink: ::grpcio::UnarySink<super::operations::ListOperationsResponse>);
    fn get_operation(&mut self, ctx: ::grpcio::RpcContext, req: super::operations::GetOperationRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn delete_operation(&mut self, ctx: ::grpcio::RpcContext, req: super::operations::DeleteOperationRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn cancel_operation(&mut self, ctx: ::grpcio::RpcContext, req: super::operations::CancelOperationRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
}

pub fn create_operations<S: Operations + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_OPERATIONS_LIST_OPERATIONS, move |ctx, req, resp| {
        instance.list_operations(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_OPERATIONS_GET_OPERATION, move |ctx, req, resp| {
        instance.get_operation(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_OPERATIONS_DELETE_OPERATION, move |ctx, req, resp| {
        instance.delete_operation(ctx, req, resp)
    });
    let mut instance = s;
    builder = builder.add_unary_handler(&METHOD_OPERATIONS_CANCEL_OPERATION, move |ctx, req, resp| {
        instance.cancel_operation(ctx, req, resp)
    });
    builder.build()
}
