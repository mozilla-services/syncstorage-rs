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

const METHOD_BIGTABLE_TABLE_SERVICE_CREATE_TABLE: ::grpcio::Method<super::bigtable_table_service_messages::CreateTableRequest, super::bigtable_table_data::Table> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.table.v1.BigtableTableService/CreateTable",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_SERVICE_LIST_TABLES: ::grpcio::Method<super::bigtable_table_service_messages::ListTablesRequest, super::bigtable_table_service_messages::ListTablesResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.table.v1.BigtableTableService/ListTables",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_SERVICE_GET_TABLE: ::grpcio::Method<super::bigtable_table_service_messages::GetTableRequest, super::bigtable_table_data::Table> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.table.v1.BigtableTableService/GetTable",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_SERVICE_DELETE_TABLE: ::grpcio::Method<super::bigtable_table_service_messages::DeleteTableRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.table.v1.BigtableTableService/DeleteTable",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_SERVICE_RENAME_TABLE: ::grpcio::Method<super::bigtable_table_service_messages::RenameTableRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.table.v1.BigtableTableService/RenameTable",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_SERVICE_CREATE_COLUMN_FAMILY: ::grpcio::Method<super::bigtable_table_service_messages::CreateColumnFamilyRequest, super::bigtable_table_data::ColumnFamily> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.table.v1.BigtableTableService/CreateColumnFamily",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_SERVICE_UPDATE_COLUMN_FAMILY: ::grpcio::Method<super::bigtable_table_data::ColumnFamily, super::bigtable_table_data::ColumnFamily> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.table.v1.BigtableTableService/UpdateColumnFamily",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_SERVICE_DELETE_COLUMN_FAMILY: ::grpcio::Method<super::bigtable_table_service_messages::DeleteColumnFamilyRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.table.v1.BigtableTableService/DeleteColumnFamily",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_SERVICE_BULK_DELETE_ROWS: ::grpcio::Method<super::bigtable_table_service_messages::BulkDeleteRowsRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.table.v1.BigtableTableService/BulkDeleteRows",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct BigtableTableServiceClient {
    client: ::grpcio::Client,
}

impl BigtableTableServiceClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        BigtableTableServiceClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn create_table_opt(&self, req: &super::bigtable_table_service_messages::CreateTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_table_data::Table> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_SERVICE_CREATE_TABLE, req, opt)
    }

    pub fn create_table(&self, req: &super::bigtable_table_service_messages::CreateTableRequest) -> ::grpcio::Result<super::bigtable_table_data::Table> {
        self.create_table_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_table_async_opt(&self, req: &super::bigtable_table_service_messages::CreateTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_data::Table>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_SERVICE_CREATE_TABLE, req, opt)
    }

    pub fn create_table_async(&self, req: &super::bigtable_table_service_messages::CreateTableRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_data::Table>> {
        self.create_table_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_tables_opt(&self, req: &super::bigtable_table_service_messages::ListTablesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_table_service_messages::ListTablesResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_SERVICE_LIST_TABLES, req, opt)
    }

    pub fn list_tables(&self, req: &super::bigtable_table_service_messages::ListTablesRequest) -> ::grpcio::Result<super::bigtable_table_service_messages::ListTablesResponse> {
        self.list_tables_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_tables_async_opt(&self, req: &super::bigtable_table_service_messages::ListTablesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_service_messages::ListTablesResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_SERVICE_LIST_TABLES, req, opt)
    }

    pub fn list_tables_async(&self, req: &super::bigtable_table_service_messages::ListTablesRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_service_messages::ListTablesResponse>> {
        self.list_tables_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_table_opt(&self, req: &super::bigtable_table_service_messages::GetTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_table_data::Table> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_SERVICE_GET_TABLE, req, opt)
    }

    pub fn get_table(&self, req: &super::bigtable_table_service_messages::GetTableRequest) -> ::grpcio::Result<super::bigtable_table_data::Table> {
        self.get_table_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_table_async_opt(&self, req: &super::bigtable_table_service_messages::GetTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_data::Table>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_SERVICE_GET_TABLE, req, opt)
    }

    pub fn get_table_async(&self, req: &super::bigtable_table_service_messages::GetTableRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_data::Table>> {
        self.get_table_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_table_opt(&self, req: &super::bigtable_table_service_messages::DeleteTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_SERVICE_DELETE_TABLE, req, opt)
    }

    pub fn delete_table(&self, req: &super::bigtable_table_service_messages::DeleteTableRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_table_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_table_async_opt(&self, req: &super::bigtable_table_service_messages::DeleteTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_SERVICE_DELETE_TABLE, req, opt)
    }

    pub fn delete_table_async(&self, req: &super::bigtable_table_service_messages::DeleteTableRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_table_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn rename_table_opt(&self, req: &super::bigtable_table_service_messages::RenameTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_SERVICE_RENAME_TABLE, req, opt)
    }

    pub fn rename_table(&self, req: &super::bigtable_table_service_messages::RenameTableRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.rename_table_opt(req, ::grpcio::CallOption::default())
    }

    pub fn rename_table_async_opt(&self, req: &super::bigtable_table_service_messages::RenameTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_SERVICE_RENAME_TABLE, req, opt)
    }

    pub fn rename_table_async(&self, req: &super::bigtable_table_service_messages::RenameTableRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.rename_table_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_column_family_opt(&self, req: &super::bigtable_table_service_messages::CreateColumnFamilyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_table_data::ColumnFamily> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_SERVICE_CREATE_COLUMN_FAMILY, req, opt)
    }

    pub fn create_column_family(&self, req: &super::bigtable_table_service_messages::CreateColumnFamilyRequest) -> ::grpcio::Result<super::bigtable_table_data::ColumnFamily> {
        self.create_column_family_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_column_family_async_opt(&self, req: &super::bigtable_table_service_messages::CreateColumnFamilyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_data::ColumnFamily>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_SERVICE_CREATE_COLUMN_FAMILY, req, opt)
    }

    pub fn create_column_family_async(&self, req: &super::bigtable_table_service_messages::CreateColumnFamilyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_data::ColumnFamily>> {
        self.create_column_family_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_column_family_opt(&self, req: &super::bigtable_table_data::ColumnFamily, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_table_data::ColumnFamily> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_SERVICE_UPDATE_COLUMN_FAMILY, req, opt)
    }

    pub fn update_column_family(&self, req: &super::bigtable_table_data::ColumnFamily) -> ::grpcio::Result<super::bigtable_table_data::ColumnFamily> {
        self.update_column_family_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_column_family_async_opt(&self, req: &super::bigtable_table_data::ColumnFamily, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_data::ColumnFamily>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_SERVICE_UPDATE_COLUMN_FAMILY, req, opt)
    }

    pub fn update_column_family_async(&self, req: &super::bigtable_table_data::ColumnFamily) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_data::ColumnFamily>> {
        self.update_column_family_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_column_family_opt(&self, req: &super::bigtable_table_service_messages::DeleteColumnFamilyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_SERVICE_DELETE_COLUMN_FAMILY, req, opt)
    }

    pub fn delete_column_family(&self, req: &super::bigtable_table_service_messages::DeleteColumnFamilyRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_column_family_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_column_family_async_opt(&self, req: &super::bigtable_table_service_messages::DeleteColumnFamilyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_SERVICE_DELETE_COLUMN_FAMILY, req, opt)
    }

    pub fn delete_column_family_async(&self, req: &super::bigtable_table_service_messages::DeleteColumnFamilyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_column_family_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn bulk_delete_rows_opt(&self, req: &super::bigtable_table_service_messages::BulkDeleteRowsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_SERVICE_BULK_DELETE_ROWS, req, opt)
    }

    pub fn bulk_delete_rows(&self, req: &super::bigtable_table_service_messages::BulkDeleteRowsRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.bulk_delete_rows_opt(req, ::grpcio::CallOption::default())
    }

    pub fn bulk_delete_rows_async_opt(&self, req: &super::bigtable_table_service_messages::BulkDeleteRowsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_SERVICE_BULK_DELETE_ROWS, req, opt)
    }

    pub fn bulk_delete_rows_async(&self, req: &super::bigtable_table_service_messages::BulkDeleteRowsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.bulk_delete_rows_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait BigtableTableService {
    fn create_table(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_service_messages::CreateTableRequest, sink: ::grpcio::UnarySink<super::bigtable_table_data::Table>);
    fn list_tables(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_service_messages::ListTablesRequest, sink: ::grpcio::UnarySink<super::bigtable_table_service_messages::ListTablesResponse>);
    fn get_table(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_service_messages::GetTableRequest, sink: ::grpcio::UnarySink<super::bigtable_table_data::Table>);
    fn delete_table(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_service_messages::DeleteTableRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn rename_table(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_service_messages::RenameTableRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn create_column_family(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_service_messages::CreateColumnFamilyRequest, sink: ::grpcio::UnarySink<super::bigtable_table_data::ColumnFamily>);
    fn update_column_family(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_data::ColumnFamily, sink: ::grpcio::UnarySink<super::bigtable_table_data::ColumnFamily>);
    fn delete_column_family(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_service_messages::DeleteColumnFamilyRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn bulk_delete_rows(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_service_messages::BulkDeleteRowsRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
}

pub fn create_bigtable_table_service<S: BigtableTableService + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_SERVICE_CREATE_TABLE, move |ctx, req, resp| {
        instance.create_table(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_SERVICE_LIST_TABLES, move |ctx, req, resp| {
        instance.list_tables(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_SERVICE_GET_TABLE, move |ctx, req, resp| {
        instance.get_table(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_SERVICE_DELETE_TABLE, move |ctx, req, resp| {
        instance.delete_table(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_SERVICE_RENAME_TABLE, move |ctx, req, resp| {
        instance.rename_table(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_SERVICE_CREATE_COLUMN_FAMILY, move |ctx, req, resp| {
        instance.create_column_family(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_SERVICE_UPDATE_COLUMN_FAMILY, move |ctx, req, resp| {
        instance.update_column_family(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_SERVICE_DELETE_COLUMN_FAMILY, move |ctx, req, resp| {
        instance.delete_column_family(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_SERVICE_BULK_DELETE_ROWS, move |ctx, req, resp| {
        instance.bulk_delete_rows(ctx, req, resp)
    });
    builder.build()
}
