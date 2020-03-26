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

const METHOD_BIGTABLE_SERVICE_READ_ROWS: ::grpcio::Method<super::bigtable_service_messages::ReadRowsRequest, super::bigtable_service_messages::ReadRowsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::ServerStreaming,
    name: "/google.bigtable.v1.BigtableService/ReadRows",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_SERVICE_SAMPLE_ROW_KEYS: ::grpcio::Method<super::bigtable_service_messages::SampleRowKeysRequest, super::bigtable_service_messages::SampleRowKeysResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::ServerStreaming,
    name: "/google.bigtable.v1.BigtableService/SampleRowKeys",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_SERVICE_MUTATE_ROW: ::grpcio::Method<super::bigtable_service_messages::MutateRowRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.v1.BigtableService/MutateRow",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_SERVICE_MUTATE_ROWS: ::grpcio::Method<super::bigtable_service_messages::MutateRowsRequest, super::bigtable_service_messages::MutateRowsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.v1.BigtableService/MutateRows",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_SERVICE_CHECK_AND_MUTATE_ROW: ::grpcio::Method<super::bigtable_service_messages::CheckAndMutateRowRequest, super::bigtable_service_messages::CheckAndMutateRowResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.v1.BigtableService/CheckAndMutateRow",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_SERVICE_READ_MODIFY_WRITE_ROW: ::grpcio::Method<super::bigtable_service_messages::ReadModifyWriteRowRequest, super::bigtable_data::Row> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.v1.BigtableService/ReadModifyWriteRow",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct BigtableServiceClient {
    client: ::grpcio::Client,
}

impl BigtableServiceClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        BigtableServiceClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn read_rows_opt(&self, req: &super::bigtable_service_messages::ReadRowsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::bigtable_service_messages::ReadRowsResponse>> {
        self.client.server_streaming(&METHOD_BIGTABLE_SERVICE_READ_ROWS, req, opt)
    }

    pub fn read_rows(&self, req: &super::bigtable_service_messages::ReadRowsRequest) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::bigtable_service_messages::ReadRowsResponse>> {
        self.read_rows_opt(req, ::grpcio::CallOption::default())
    }

    pub fn sample_row_keys_opt(&self, req: &super::bigtable_service_messages::SampleRowKeysRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::bigtable_service_messages::SampleRowKeysResponse>> {
        self.client.server_streaming(&METHOD_BIGTABLE_SERVICE_SAMPLE_ROW_KEYS, req, opt)
    }

    pub fn sample_row_keys(&self, req: &super::bigtable_service_messages::SampleRowKeysRequest) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::bigtable_service_messages::SampleRowKeysResponse>> {
        self.sample_row_keys_opt(req, ::grpcio::CallOption::default())
    }

    pub fn mutate_row_opt(&self, req: &super::bigtable_service_messages::MutateRowRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_BIGTABLE_SERVICE_MUTATE_ROW, req, opt)
    }

    pub fn mutate_row(&self, req: &super::bigtable_service_messages::MutateRowRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.mutate_row_opt(req, ::grpcio::CallOption::default())
    }

    pub fn mutate_row_async_opt(&self, req: &super::bigtable_service_messages::MutateRowRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_SERVICE_MUTATE_ROW, req, opt)
    }

    pub fn mutate_row_async(&self, req: &super::bigtable_service_messages::MutateRowRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.mutate_row_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn mutate_rows_opt(&self, req: &super::bigtable_service_messages::MutateRowsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_service_messages::MutateRowsResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_SERVICE_MUTATE_ROWS, req, opt)
    }

    pub fn mutate_rows(&self, req: &super::bigtable_service_messages::MutateRowsRequest) -> ::grpcio::Result<super::bigtable_service_messages::MutateRowsResponse> {
        self.mutate_rows_opt(req, ::grpcio::CallOption::default())
    }

    pub fn mutate_rows_async_opt(&self, req: &super::bigtable_service_messages::MutateRowsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_service_messages::MutateRowsResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_SERVICE_MUTATE_ROWS, req, opt)
    }

    pub fn mutate_rows_async(&self, req: &super::bigtable_service_messages::MutateRowsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_service_messages::MutateRowsResponse>> {
        self.mutate_rows_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn check_and_mutate_row_opt(&self, req: &super::bigtable_service_messages::CheckAndMutateRowRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_service_messages::CheckAndMutateRowResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_SERVICE_CHECK_AND_MUTATE_ROW, req, opt)
    }

    pub fn check_and_mutate_row(&self, req: &super::bigtable_service_messages::CheckAndMutateRowRequest) -> ::grpcio::Result<super::bigtable_service_messages::CheckAndMutateRowResponse> {
        self.check_and_mutate_row_opt(req, ::grpcio::CallOption::default())
    }

    pub fn check_and_mutate_row_async_opt(&self, req: &super::bigtable_service_messages::CheckAndMutateRowRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_service_messages::CheckAndMutateRowResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_SERVICE_CHECK_AND_MUTATE_ROW, req, opt)
    }

    pub fn check_and_mutate_row_async(&self, req: &super::bigtable_service_messages::CheckAndMutateRowRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_service_messages::CheckAndMutateRowResponse>> {
        self.check_and_mutate_row_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn read_modify_write_row_opt(&self, req: &super::bigtable_service_messages::ReadModifyWriteRowRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_data::Row> {
        self.client.unary_call(&METHOD_BIGTABLE_SERVICE_READ_MODIFY_WRITE_ROW, req, opt)
    }

    pub fn read_modify_write_row(&self, req: &super::bigtable_service_messages::ReadModifyWriteRowRequest) -> ::grpcio::Result<super::bigtable_data::Row> {
        self.read_modify_write_row_opt(req, ::grpcio::CallOption::default())
    }

    pub fn read_modify_write_row_async_opt(&self, req: &super::bigtable_service_messages::ReadModifyWriteRowRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_data::Row>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_SERVICE_READ_MODIFY_WRITE_ROW, req, opt)
    }

    pub fn read_modify_write_row_async(&self, req: &super::bigtable_service_messages::ReadModifyWriteRowRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_data::Row>> {
        self.read_modify_write_row_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait BigtableService {
    fn read_rows(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_service_messages::ReadRowsRequest, sink: ::grpcio::ServerStreamingSink<super::bigtable_service_messages::ReadRowsResponse>);
    fn sample_row_keys(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_service_messages::SampleRowKeysRequest, sink: ::grpcio::ServerStreamingSink<super::bigtable_service_messages::SampleRowKeysResponse>);
    fn mutate_row(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_service_messages::MutateRowRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn mutate_rows(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_service_messages::MutateRowsRequest, sink: ::grpcio::UnarySink<super::bigtable_service_messages::MutateRowsResponse>);
    fn check_and_mutate_row(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_service_messages::CheckAndMutateRowRequest, sink: ::grpcio::UnarySink<super::bigtable_service_messages::CheckAndMutateRowResponse>);
    fn read_modify_write_row(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_service_messages::ReadModifyWriteRowRequest, sink: ::grpcio::UnarySink<super::bigtable_data::Row>);
}

pub fn create_bigtable_service<S: BigtableService + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_server_streaming_handler(&METHOD_BIGTABLE_SERVICE_READ_ROWS, move |ctx, req, resp| {
        instance.read_rows(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_server_streaming_handler(&METHOD_BIGTABLE_SERVICE_SAMPLE_ROW_KEYS, move |ctx, req, resp| {
        instance.sample_row_keys(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_SERVICE_MUTATE_ROW, move |ctx, req, resp| {
        instance.mutate_row(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_SERVICE_MUTATE_ROWS, move |ctx, req, resp| {
        instance.mutate_rows(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_SERVICE_CHECK_AND_MUTATE_ROW, move |ctx, req, resp| {
        instance.check_and_mutate_row(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_SERVICE_READ_MODIFY_WRITE_ROW, move |ctx, req, resp| {
        instance.read_modify_write_row(ctx, req, resp)
    });
    builder.build()
}
