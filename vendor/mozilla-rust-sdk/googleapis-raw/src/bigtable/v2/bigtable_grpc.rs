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

const METHOD_BIGTABLE_READ_ROWS: ::grpcio::Method<super::bigtable::ReadRowsRequest, super::bigtable::ReadRowsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::ServerStreaming,
    name: "/google.bigtable.v2.Bigtable/ReadRows",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_SAMPLE_ROW_KEYS: ::grpcio::Method<super::bigtable::SampleRowKeysRequest, super::bigtable::SampleRowKeysResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::ServerStreaming,
    name: "/google.bigtable.v2.Bigtable/SampleRowKeys",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_MUTATE_ROW: ::grpcio::Method<super::bigtable::MutateRowRequest, super::bigtable::MutateRowResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.v2.Bigtable/MutateRow",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_MUTATE_ROWS: ::grpcio::Method<super::bigtable::MutateRowsRequest, super::bigtable::MutateRowsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::ServerStreaming,
    name: "/google.bigtable.v2.Bigtable/MutateRows",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_CHECK_AND_MUTATE_ROW: ::grpcio::Method<super::bigtable::CheckAndMutateRowRequest, super::bigtable::CheckAndMutateRowResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.v2.Bigtable/CheckAndMutateRow",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_READ_MODIFY_WRITE_ROW: ::grpcio::Method<super::bigtable::ReadModifyWriteRowRequest, super::bigtable::ReadModifyWriteRowResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.v2.Bigtable/ReadModifyWriteRow",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct BigtableClient {
    client: ::grpcio::Client,
}

impl BigtableClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        BigtableClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn read_rows_opt(&self, req: &super::bigtable::ReadRowsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::bigtable::ReadRowsResponse>> {
        self.client.server_streaming(&METHOD_BIGTABLE_READ_ROWS, req, opt)
    }

    pub fn read_rows(&self, req: &super::bigtable::ReadRowsRequest) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::bigtable::ReadRowsResponse>> {
        self.read_rows_opt(req, ::grpcio::CallOption::default())
    }

    pub fn sample_row_keys_opt(&self, req: &super::bigtable::SampleRowKeysRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::bigtable::SampleRowKeysResponse>> {
        self.client.server_streaming(&METHOD_BIGTABLE_SAMPLE_ROW_KEYS, req, opt)
    }

    pub fn sample_row_keys(&self, req: &super::bigtable::SampleRowKeysRequest) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::bigtable::SampleRowKeysResponse>> {
        self.sample_row_keys_opt(req, ::grpcio::CallOption::default())
    }

    pub fn mutate_row_opt(&self, req: &super::bigtable::MutateRowRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable::MutateRowResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_MUTATE_ROW, req, opt)
    }

    pub fn mutate_row(&self, req: &super::bigtable::MutateRowRequest) -> ::grpcio::Result<super::bigtable::MutateRowResponse> {
        self.mutate_row_opt(req, ::grpcio::CallOption::default())
    }

    pub fn mutate_row_async_opt(&self, req: &super::bigtable::MutateRowRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable::MutateRowResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_MUTATE_ROW, req, opt)
    }

    pub fn mutate_row_async(&self, req: &super::bigtable::MutateRowRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable::MutateRowResponse>> {
        self.mutate_row_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn mutate_rows_opt(&self, req: &super::bigtable::MutateRowsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::bigtable::MutateRowsResponse>> {
        self.client.server_streaming(&METHOD_BIGTABLE_MUTATE_ROWS, req, opt)
    }

    pub fn mutate_rows(&self, req: &super::bigtable::MutateRowsRequest) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::bigtable::MutateRowsResponse>> {
        self.mutate_rows_opt(req, ::grpcio::CallOption::default())
    }

    pub fn check_and_mutate_row_opt(&self, req: &super::bigtable::CheckAndMutateRowRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable::CheckAndMutateRowResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_CHECK_AND_MUTATE_ROW, req, opt)
    }

    pub fn check_and_mutate_row(&self, req: &super::bigtable::CheckAndMutateRowRequest) -> ::grpcio::Result<super::bigtable::CheckAndMutateRowResponse> {
        self.check_and_mutate_row_opt(req, ::grpcio::CallOption::default())
    }

    pub fn check_and_mutate_row_async_opt(&self, req: &super::bigtable::CheckAndMutateRowRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable::CheckAndMutateRowResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_CHECK_AND_MUTATE_ROW, req, opt)
    }

    pub fn check_and_mutate_row_async(&self, req: &super::bigtable::CheckAndMutateRowRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable::CheckAndMutateRowResponse>> {
        self.check_and_mutate_row_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn read_modify_write_row_opt(&self, req: &super::bigtable::ReadModifyWriteRowRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable::ReadModifyWriteRowResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_READ_MODIFY_WRITE_ROW, req, opt)
    }

    pub fn read_modify_write_row(&self, req: &super::bigtable::ReadModifyWriteRowRequest) -> ::grpcio::Result<super::bigtable::ReadModifyWriteRowResponse> {
        self.read_modify_write_row_opt(req, ::grpcio::CallOption::default())
    }

    pub fn read_modify_write_row_async_opt(&self, req: &super::bigtable::ReadModifyWriteRowRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable::ReadModifyWriteRowResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_READ_MODIFY_WRITE_ROW, req, opt)
    }

    pub fn read_modify_write_row_async(&self, req: &super::bigtable::ReadModifyWriteRowRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable::ReadModifyWriteRowResponse>> {
        self.read_modify_write_row_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait Bigtable {
    fn read_rows(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable::ReadRowsRequest, sink: ::grpcio::ServerStreamingSink<super::bigtable::ReadRowsResponse>);
    fn sample_row_keys(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable::SampleRowKeysRequest, sink: ::grpcio::ServerStreamingSink<super::bigtable::SampleRowKeysResponse>);
    fn mutate_row(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable::MutateRowRequest, sink: ::grpcio::UnarySink<super::bigtable::MutateRowResponse>);
    fn mutate_rows(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable::MutateRowsRequest, sink: ::grpcio::ServerStreamingSink<super::bigtable::MutateRowsResponse>);
    fn check_and_mutate_row(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable::CheckAndMutateRowRequest, sink: ::grpcio::UnarySink<super::bigtable::CheckAndMutateRowResponse>);
    fn read_modify_write_row(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable::ReadModifyWriteRowRequest, sink: ::grpcio::UnarySink<super::bigtable::ReadModifyWriteRowResponse>);
}

pub fn create_bigtable<S: Bigtable + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_server_streaming_handler(&METHOD_BIGTABLE_READ_ROWS, move |ctx, req, resp| {
        instance.read_rows(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_server_streaming_handler(&METHOD_BIGTABLE_SAMPLE_ROW_KEYS, move |ctx, req, resp| {
        instance.sample_row_keys(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_MUTATE_ROW, move |ctx, req, resp| {
        instance.mutate_row(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_server_streaming_handler(&METHOD_BIGTABLE_MUTATE_ROWS, move |ctx, req, resp| {
        instance.mutate_rows(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_CHECK_AND_MUTATE_ROW, move |ctx, req, resp| {
        instance.check_and_mutate_row(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_READ_MODIFY_WRITE_ROW, move |ctx, req, resp| {
        instance.read_modify_write_row(ctx, req, resp)
    });
    builder.build()
}
