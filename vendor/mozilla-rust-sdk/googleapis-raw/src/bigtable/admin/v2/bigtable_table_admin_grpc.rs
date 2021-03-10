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

const METHOD_BIGTABLE_TABLE_ADMIN_CREATE_TABLE: ::grpcio::Method<super::bigtable_table_admin::CreateTableRequest, super::table::Table> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/CreateTable",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_CREATE_TABLE_FROM_SNAPSHOT: ::grpcio::Method<super::bigtable_table_admin::CreateTableFromSnapshotRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/CreateTableFromSnapshot",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_LIST_TABLES: ::grpcio::Method<super::bigtable_table_admin::ListTablesRequest, super::bigtable_table_admin::ListTablesResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/ListTables",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_GET_TABLE: ::grpcio::Method<super::bigtable_table_admin::GetTableRequest, super::table::Table> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/GetTable",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_DELETE_TABLE: ::grpcio::Method<super::bigtable_table_admin::DeleteTableRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/DeleteTable",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_MODIFY_COLUMN_FAMILIES: ::grpcio::Method<super::bigtable_table_admin::ModifyColumnFamiliesRequest, super::table::Table> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/ModifyColumnFamilies",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_DROP_ROW_RANGE: ::grpcio::Method<super::bigtable_table_admin::DropRowRangeRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/DropRowRange",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_GENERATE_CONSISTENCY_TOKEN: ::grpcio::Method<super::bigtable_table_admin::GenerateConsistencyTokenRequest, super::bigtable_table_admin::GenerateConsistencyTokenResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/GenerateConsistencyToken",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_CHECK_CONSISTENCY: ::grpcio::Method<super::bigtable_table_admin::CheckConsistencyRequest, super::bigtable_table_admin::CheckConsistencyResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/CheckConsistency",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_SNAPSHOT_TABLE: ::grpcio::Method<super::bigtable_table_admin::SnapshotTableRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/SnapshotTable",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_GET_SNAPSHOT: ::grpcio::Method<super::bigtable_table_admin::GetSnapshotRequest, super::table::Snapshot> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/GetSnapshot",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_LIST_SNAPSHOTS: ::grpcio::Method<super::bigtable_table_admin::ListSnapshotsRequest, super::bigtable_table_admin::ListSnapshotsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/ListSnapshots",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_DELETE_SNAPSHOT: ::grpcio::Method<super::bigtable_table_admin::DeleteSnapshotRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/DeleteSnapshot",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_CREATE_BACKUP: ::grpcio::Method<super::bigtable_table_admin::CreateBackupRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/CreateBackup",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_GET_BACKUP: ::grpcio::Method<super::bigtable_table_admin::GetBackupRequest, super::table::Backup> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/GetBackup",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_UPDATE_BACKUP: ::grpcio::Method<super::bigtable_table_admin::UpdateBackupRequest, super::table::Backup> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/UpdateBackup",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_DELETE_BACKUP: ::grpcio::Method<super::bigtable_table_admin::DeleteBackupRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/DeleteBackup",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_LIST_BACKUPS: ::grpcio::Method<super::bigtable_table_admin::ListBackupsRequest, super::bigtable_table_admin::ListBackupsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/ListBackups",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_RESTORE_TABLE: ::grpcio::Method<super::bigtable_table_admin::RestoreTableRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/RestoreTable",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_GET_IAM_POLICY: ::grpcio::Method<super::iam_policy::GetIamPolicyRequest, super::policy::Policy> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/GetIamPolicy",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_SET_IAM_POLICY: ::grpcio::Method<super::iam_policy::SetIamPolicyRequest, super::policy::Policy> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/SetIamPolicy",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_TABLE_ADMIN_TEST_IAM_PERMISSIONS: ::grpcio::Method<super::iam_policy::TestIamPermissionsRequest, super::iam_policy::TestIamPermissionsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.v2.BigtableTableAdmin/TestIamPermissions",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct BigtableTableAdminClient {
    client: ::grpcio::Client,
}

impl BigtableTableAdminClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        BigtableTableAdminClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn create_table_opt(&self, req: &super::bigtable_table_admin::CreateTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::table::Table> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_CREATE_TABLE, req, opt)
    }

    pub fn create_table(&self, req: &super::bigtable_table_admin::CreateTableRequest) -> ::grpcio::Result<super::table::Table> {
        self.create_table_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_table_async_opt(&self, req: &super::bigtable_table_admin::CreateTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::table::Table>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_CREATE_TABLE, req, opt)
    }

    pub fn create_table_async(&self, req: &super::bigtable_table_admin::CreateTableRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::table::Table>> {
        self.create_table_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_table_from_snapshot_opt(&self, req: &super::bigtable_table_admin::CreateTableFromSnapshotRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_CREATE_TABLE_FROM_SNAPSHOT, req, opt)
    }

    pub fn create_table_from_snapshot(&self, req: &super::bigtable_table_admin::CreateTableFromSnapshotRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.create_table_from_snapshot_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_table_from_snapshot_async_opt(&self, req: &super::bigtable_table_admin::CreateTableFromSnapshotRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_CREATE_TABLE_FROM_SNAPSHOT, req, opt)
    }

    pub fn create_table_from_snapshot_async(&self, req: &super::bigtable_table_admin::CreateTableFromSnapshotRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.create_table_from_snapshot_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_tables_opt(&self, req: &super::bigtable_table_admin::ListTablesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_table_admin::ListTablesResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_LIST_TABLES, req, opt)
    }

    pub fn list_tables(&self, req: &super::bigtable_table_admin::ListTablesRequest) -> ::grpcio::Result<super::bigtable_table_admin::ListTablesResponse> {
        self.list_tables_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_tables_async_opt(&self, req: &super::bigtable_table_admin::ListTablesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_admin::ListTablesResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_LIST_TABLES, req, opt)
    }

    pub fn list_tables_async(&self, req: &super::bigtable_table_admin::ListTablesRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_admin::ListTablesResponse>> {
        self.list_tables_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_table_opt(&self, req: &super::bigtable_table_admin::GetTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::table::Table> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_GET_TABLE, req, opt)
    }

    pub fn get_table(&self, req: &super::bigtable_table_admin::GetTableRequest) -> ::grpcio::Result<super::table::Table> {
        self.get_table_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_table_async_opt(&self, req: &super::bigtable_table_admin::GetTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::table::Table>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_GET_TABLE, req, opt)
    }

    pub fn get_table_async(&self, req: &super::bigtable_table_admin::GetTableRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::table::Table>> {
        self.get_table_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_table_opt(&self, req: &super::bigtable_table_admin::DeleteTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_DELETE_TABLE, req, opt)
    }

    pub fn delete_table(&self, req: &super::bigtable_table_admin::DeleteTableRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_table_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_table_async_opt(&self, req: &super::bigtable_table_admin::DeleteTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_DELETE_TABLE, req, opt)
    }

    pub fn delete_table_async(&self, req: &super::bigtable_table_admin::DeleteTableRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_table_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn modify_column_families_opt(&self, req: &super::bigtable_table_admin::ModifyColumnFamiliesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::table::Table> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_MODIFY_COLUMN_FAMILIES, req, opt)
    }

    pub fn modify_column_families(&self, req: &super::bigtable_table_admin::ModifyColumnFamiliesRequest) -> ::grpcio::Result<super::table::Table> {
        self.modify_column_families_opt(req, ::grpcio::CallOption::default())
    }

    pub fn modify_column_families_async_opt(&self, req: &super::bigtable_table_admin::ModifyColumnFamiliesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::table::Table>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_MODIFY_COLUMN_FAMILIES, req, opt)
    }

    pub fn modify_column_families_async(&self, req: &super::bigtable_table_admin::ModifyColumnFamiliesRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::table::Table>> {
        self.modify_column_families_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn drop_row_range_opt(&self, req: &super::bigtable_table_admin::DropRowRangeRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_DROP_ROW_RANGE, req, opt)
    }

    pub fn drop_row_range(&self, req: &super::bigtable_table_admin::DropRowRangeRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.drop_row_range_opt(req, ::grpcio::CallOption::default())
    }

    pub fn drop_row_range_async_opt(&self, req: &super::bigtable_table_admin::DropRowRangeRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_DROP_ROW_RANGE, req, opt)
    }

    pub fn drop_row_range_async(&self, req: &super::bigtable_table_admin::DropRowRangeRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.drop_row_range_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn generate_consistency_token_opt(&self, req: &super::bigtable_table_admin::GenerateConsistencyTokenRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_table_admin::GenerateConsistencyTokenResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_GENERATE_CONSISTENCY_TOKEN, req, opt)
    }

    pub fn generate_consistency_token(&self, req: &super::bigtable_table_admin::GenerateConsistencyTokenRequest) -> ::grpcio::Result<super::bigtable_table_admin::GenerateConsistencyTokenResponse> {
        self.generate_consistency_token_opt(req, ::grpcio::CallOption::default())
    }

    pub fn generate_consistency_token_async_opt(&self, req: &super::bigtable_table_admin::GenerateConsistencyTokenRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_admin::GenerateConsistencyTokenResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_GENERATE_CONSISTENCY_TOKEN, req, opt)
    }

    pub fn generate_consistency_token_async(&self, req: &super::bigtable_table_admin::GenerateConsistencyTokenRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_admin::GenerateConsistencyTokenResponse>> {
        self.generate_consistency_token_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn check_consistency_opt(&self, req: &super::bigtable_table_admin::CheckConsistencyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_table_admin::CheckConsistencyResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_CHECK_CONSISTENCY, req, opt)
    }

    pub fn check_consistency(&self, req: &super::bigtable_table_admin::CheckConsistencyRequest) -> ::grpcio::Result<super::bigtable_table_admin::CheckConsistencyResponse> {
        self.check_consistency_opt(req, ::grpcio::CallOption::default())
    }

    pub fn check_consistency_async_opt(&self, req: &super::bigtable_table_admin::CheckConsistencyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_admin::CheckConsistencyResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_CHECK_CONSISTENCY, req, opt)
    }

    pub fn check_consistency_async(&self, req: &super::bigtable_table_admin::CheckConsistencyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_admin::CheckConsistencyResponse>> {
        self.check_consistency_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn snapshot_table_opt(&self, req: &super::bigtable_table_admin::SnapshotTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_SNAPSHOT_TABLE, req, opt)
    }

    pub fn snapshot_table(&self, req: &super::bigtable_table_admin::SnapshotTableRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.snapshot_table_opt(req, ::grpcio::CallOption::default())
    }

    pub fn snapshot_table_async_opt(&self, req: &super::bigtable_table_admin::SnapshotTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_SNAPSHOT_TABLE, req, opt)
    }

    pub fn snapshot_table_async(&self, req: &super::bigtable_table_admin::SnapshotTableRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.snapshot_table_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_snapshot_opt(&self, req: &super::bigtable_table_admin::GetSnapshotRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::table::Snapshot> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_GET_SNAPSHOT, req, opt)
    }

    pub fn get_snapshot(&self, req: &super::bigtable_table_admin::GetSnapshotRequest) -> ::grpcio::Result<super::table::Snapshot> {
        self.get_snapshot_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_snapshot_async_opt(&self, req: &super::bigtable_table_admin::GetSnapshotRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::table::Snapshot>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_GET_SNAPSHOT, req, opt)
    }

    pub fn get_snapshot_async(&self, req: &super::bigtable_table_admin::GetSnapshotRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::table::Snapshot>> {
        self.get_snapshot_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_snapshots_opt(&self, req: &super::bigtable_table_admin::ListSnapshotsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_table_admin::ListSnapshotsResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_LIST_SNAPSHOTS, req, opt)
    }

    pub fn list_snapshots(&self, req: &super::bigtable_table_admin::ListSnapshotsRequest) -> ::grpcio::Result<super::bigtable_table_admin::ListSnapshotsResponse> {
        self.list_snapshots_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_snapshots_async_opt(&self, req: &super::bigtable_table_admin::ListSnapshotsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_admin::ListSnapshotsResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_LIST_SNAPSHOTS, req, opt)
    }

    pub fn list_snapshots_async(&self, req: &super::bigtable_table_admin::ListSnapshotsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_admin::ListSnapshotsResponse>> {
        self.list_snapshots_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_snapshot_opt(&self, req: &super::bigtable_table_admin::DeleteSnapshotRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_DELETE_SNAPSHOT, req, opt)
    }

    pub fn delete_snapshot(&self, req: &super::bigtable_table_admin::DeleteSnapshotRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_snapshot_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_snapshot_async_opt(&self, req: &super::bigtable_table_admin::DeleteSnapshotRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_DELETE_SNAPSHOT, req, opt)
    }

    pub fn delete_snapshot_async(&self, req: &super::bigtable_table_admin::DeleteSnapshotRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_snapshot_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_backup_opt(&self, req: &super::bigtable_table_admin::CreateBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_CREATE_BACKUP, req, opt)
    }

    pub fn create_backup(&self, req: &super::bigtable_table_admin::CreateBackupRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.create_backup_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_backup_async_opt(&self, req: &super::bigtable_table_admin::CreateBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_CREATE_BACKUP, req, opt)
    }

    pub fn create_backup_async(&self, req: &super::bigtable_table_admin::CreateBackupRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.create_backup_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_backup_opt(&self, req: &super::bigtable_table_admin::GetBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::table::Backup> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_GET_BACKUP, req, opt)
    }

    pub fn get_backup(&self, req: &super::bigtable_table_admin::GetBackupRequest) -> ::grpcio::Result<super::table::Backup> {
        self.get_backup_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_backup_async_opt(&self, req: &super::bigtable_table_admin::GetBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::table::Backup>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_GET_BACKUP, req, opt)
    }

    pub fn get_backup_async(&self, req: &super::bigtable_table_admin::GetBackupRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::table::Backup>> {
        self.get_backup_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_backup_opt(&self, req: &super::bigtable_table_admin::UpdateBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::table::Backup> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_UPDATE_BACKUP, req, opt)
    }

    pub fn update_backup(&self, req: &super::bigtable_table_admin::UpdateBackupRequest) -> ::grpcio::Result<super::table::Backup> {
        self.update_backup_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_backup_async_opt(&self, req: &super::bigtable_table_admin::UpdateBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::table::Backup>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_UPDATE_BACKUP, req, opt)
    }

    pub fn update_backup_async(&self, req: &super::bigtable_table_admin::UpdateBackupRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::table::Backup>> {
        self.update_backup_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_backup_opt(&self, req: &super::bigtable_table_admin::DeleteBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_DELETE_BACKUP, req, opt)
    }

    pub fn delete_backup(&self, req: &super::bigtable_table_admin::DeleteBackupRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_backup_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_backup_async_opt(&self, req: &super::bigtable_table_admin::DeleteBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_DELETE_BACKUP, req, opt)
    }

    pub fn delete_backup_async(&self, req: &super::bigtable_table_admin::DeleteBackupRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_backup_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_backups_opt(&self, req: &super::bigtable_table_admin::ListBackupsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_table_admin::ListBackupsResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_LIST_BACKUPS, req, opt)
    }

    pub fn list_backups(&self, req: &super::bigtable_table_admin::ListBackupsRequest) -> ::grpcio::Result<super::bigtable_table_admin::ListBackupsResponse> {
        self.list_backups_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_backups_async_opt(&self, req: &super::bigtable_table_admin::ListBackupsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_admin::ListBackupsResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_LIST_BACKUPS, req, opt)
    }

    pub fn list_backups_async(&self, req: &super::bigtable_table_admin::ListBackupsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_table_admin::ListBackupsResponse>> {
        self.list_backups_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn restore_table_opt(&self, req: &super::bigtable_table_admin::RestoreTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_RESTORE_TABLE, req, opt)
    }

    pub fn restore_table(&self, req: &super::bigtable_table_admin::RestoreTableRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.restore_table_opt(req, ::grpcio::CallOption::default())
    }

    pub fn restore_table_async_opt(&self, req: &super::bigtable_table_admin::RestoreTableRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_RESTORE_TABLE, req, opt)
    }

    pub fn restore_table_async(&self, req: &super::bigtable_table_admin::RestoreTableRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.restore_table_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_iam_policy_opt(&self, req: &super::iam_policy::GetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::policy::Policy> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_GET_IAM_POLICY, req, opt)
    }

    pub fn get_iam_policy(&self, req: &super::iam_policy::GetIamPolicyRequest) -> ::grpcio::Result<super::policy::Policy> {
        self.get_iam_policy_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_iam_policy_async_opt(&self, req: &super::iam_policy::GetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_GET_IAM_POLICY, req, opt)
    }

    pub fn get_iam_policy_async(&self, req: &super::iam_policy::GetIamPolicyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.get_iam_policy_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_iam_policy_opt(&self, req: &super::iam_policy::SetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::policy::Policy> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_SET_IAM_POLICY, req, opt)
    }

    pub fn set_iam_policy(&self, req: &super::iam_policy::SetIamPolicyRequest) -> ::grpcio::Result<super::policy::Policy> {
        self.set_iam_policy_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_iam_policy_async_opt(&self, req: &super::iam_policy::SetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_SET_IAM_POLICY, req, opt)
    }

    pub fn set_iam_policy_async(&self, req: &super::iam_policy::SetIamPolicyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.set_iam_policy_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn test_iam_permissions_opt(&self, req: &super::iam_policy::TestIamPermissionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::iam_policy::TestIamPermissionsResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_TABLE_ADMIN_TEST_IAM_PERMISSIONS, req, opt)
    }

    pub fn test_iam_permissions(&self, req: &super::iam_policy::TestIamPermissionsRequest) -> ::grpcio::Result<super::iam_policy::TestIamPermissionsResponse> {
        self.test_iam_permissions_opt(req, ::grpcio::CallOption::default())
    }

    pub fn test_iam_permissions_async_opt(&self, req: &super::iam_policy::TestIamPermissionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::iam_policy::TestIamPermissionsResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_TABLE_ADMIN_TEST_IAM_PERMISSIONS, req, opt)
    }

    pub fn test_iam_permissions_async(&self, req: &super::iam_policy::TestIamPermissionsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::iam_policy::TestIamPermissionsResponse>> {
        self.test_iam_permissions_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Output = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait BigtableTableAdmin {
    fn create_table(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::CreateTableRequest, sink: ::grpcio::UnarySink<super::table::Table>);
    fn create_table_from_snapshot(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::CreateTableFromSnapshotRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn list_tables(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::ListTablesRequest, sink: ::grpcio::UnarySink<super::bigtable_table_admin::ListTablesResponse>);
    fn get_table(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::GetTableRequest, sink: ::grpcio::UnarySink<super::table::Table>);
    fn delete_table(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::DeleteTableRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn modify_column_families(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::ModifyColumnFamiliesRequest, sink: ::grpcio::UnarySink<super::table::Table>);
    fn drop_row_range(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::DropRowRangeRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn generate_consistency_token(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::GenerateConsistencyTokenRequest, sink: ::grpcio::UnarySink<super::bigtable_table_admin::GenerateConsistencyTokenResponse>);
    fn check_consistency(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::CheckConsistencyRequest, sink: ::grpcio::UnarySink<super::bigtable_table_admin::CheckConsistencyResponse>);
    fn snapshot_table(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::SnapshotTableRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn get_snapshot(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::GetSnapshotRequest, sink: ::grpcio::UnarySink<super::table::Snapshot>);
    fn list_snapshots(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::ListSnapshotsRequest, sink: ::grpcio::UnarySink<super::bigtable_table_admin::ListSnapshotsResponse>);
    fn delete_snapshot(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::DeleteSnapshotRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn create_backup(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::CreateBackupRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn get_backup(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::GetBackupRequest, sink: ::grpcio::UnarySink<super::table::Backup>);
    fn update_backup(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::UpdateBackupRequest, sink: ::grpcio::UnarySink<super::table::Backup>);
    fn delete_backup(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::DeleteBackupRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn list_backups(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::ListBackupsRequest, sink: ::grpcio::UnarySink<super::bigtable_table_admin::ListBackupsResponse>);
    fn restore_table(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_table_admin::RestoreTableRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn get_iam_policy(&mut self, ctx: ::grpcio::RpcContext, req: super::iam_policy::GetIamPolicyRequest, sink: ::grpcio::UnarySink<super::policy::Policy>);
    fn set_iam_policy(&mut self, ctx: ::grpcio::RpcContext, req: super::iam_policy::SetIamPolicyRequest, sink: ::grpcio::UnarySink<super::policy::Policy>);
    fn test_iam_permissions(&mut self, ctx: ::grpcio::RpcContext, req: super::iam_policy::TestIamPermissionsRequest, sink: ::grpcio::UnarySink<super::iam_policy::TestIamPermissionsResponse>);
}

pub fn create_bigtable_table_admin<S: BigtableTableAdmin + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_CREATE_TABLE, move |ctx, req, resp| {
        instance.create_table(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_CREATE_TABLE_FROM_SNAPSHOT, move |ctx, req, resp| {
        instance.create_table_from_snapshot(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_LIST_TABLES, move |ctx, req, resp| {
        instance.list_tables(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_GET_TABLE, move |ctx, req, resp| {
        instance.get_table(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_DELETE_TABLE, move |ctx, req, resp| {
        instance.delete_table(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_MODIFY_COLUMN_FAMILIES, move |ctx, req, resp| {
        instance.modify_column_families(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_DROP_ROW_RANGE, move |ctx, req, resp| {
        instance.drop_row_range(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_GENERATE_CONSISTENCY_TOKEN, move |ctx, req, resp| {
        instance.generate_consistency_token(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_CHECK_CONSISTENCY, move |ctx, req, resp| {
        instance.check_consistency(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_SNAPSHOT_TABLE, move |ctx, req, resp| {
        instance.snapshot_table(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_GET_SNAPSHOT, move |ctx, req, resp| {
        instance.get_snapshot(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_LIST_SNAPSHOTS, move |ctx, req, resp| {
        instance.list_snapshots(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_DELETE_SNAPSHOT, move |ctx, req, resp| {
        instance.delete_snapshot(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_CREATE_BACKUP, move |ctx, req, resp| {
        instance.create_backup(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_GET_BACKUP, move |ctx, req, resp| {
        instance.get_backup(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_UPDATE_BACKUP, move |ctx, req, resp| {
        instance.update_backup(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_DELETE_BACKUP, move |ctx, req, resp| {
        instance.delete_backup(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_LIST_BACKUPS, move |ctx, req, resp| {
        instance.list_backups(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_RESTORE_TABLE, move |ctx, req, resp| {
        instance.restore_table(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_GET_IAM_POLICY, move |ctx, req, resp| {
        instance.get_iam_policy(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_SET_IAM_POLICY, move |ctx, req, resp| {
        instance.set_iam_policy(ctx, req, resp)
    });
    let mut instance = s;
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_TABLE_ADMIN_TEST_IAM_PERMISSIONS, move |ctx, req, resp| {
        instance.test_iam_permissions(ctx, req, resp)
    });
    builder.build()
}
