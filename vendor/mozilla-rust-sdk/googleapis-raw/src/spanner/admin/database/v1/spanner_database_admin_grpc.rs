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

const METHOD_DATABASE_ADMIN_LIST_DATABASES: ::grpcio::Method<super::spanner_database_admin::ListDatabasesRequest, super::spanner_database_admin::ListDatabasesResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/ListDatabases",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_CREATE_DATABASE: ::grpcio::Method<super::spanner_database_admin::CreateDatabaseRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/CreateDatabase",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_GET_DATABASE: ::grpcio::Method<super::spanner_database_admin::GetDatabaseRequest, super::spanner_database_admin::Database> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/GetDatabase",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_UPDATE_DATABASE_DDL: ::grpcio::Method<super::spanner_database_admin::UpdateDatabaseDdlRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/UpdateDatabaseDdl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_DROP_DATABASE: ::grpcio::Method<super::spanner_database_admin::DropDatabaseRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/DropDatabase",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_GET_DATABASE_DDL: ::grpcio::Method<super::spanner_database_admin::GetDatabaseDdlRequest, super::spanner_database_admin::GetDatabaseDdlResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/GetDatabaseDdl",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_SET_IAM_POLICY: ::grpcio::Method<super::iam_policy::SetIamPolicyRequest, super::policy::Policy> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/SetIamPolicy",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_GET_IAM_POLICY: ::grpcio::Method<super::iam_policy::GetIamPolicyRequest, super::policy::Policy> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/GetIamPolicy",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_TEST_IAM_PERMISSIONS: ::grpcio::Method<super::iam_policy::TestIamPermissionsRequest, super::iam_policy::TestIamPermissionsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/TestIamPermissions",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_CREATE_BACKUP: ::grpcio::Method<super::backup::CreateBackupRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/CreateBackup",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_GET_BACKUP: ::grpcio::Method<super::backup::GetBackupRequest, super::backup::Backup> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/GetBackup",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_UPDATE_BACKUP: ::grpcio::Method<super::backup::UpdateBackupRequest, super::backup::Backup> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/UpdateBackup",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_DELETE_BACKUP: ::grpcio::Method<super::backup::DeleteBackupRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/DeleteBackup",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_LIST_BACKUPS: ::grpcio::Method<super::backup::ListBackupsRequest, super::backup::ListBackupsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/ListBackups",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_RESTORE_DATABASE: ::grpcio::Method<super::spanner_database_admin::RestoreDatabaseRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/RestoreDatabase",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_LIST_DATABASE_OPERATIONS: ::grpcio::Method<super::spanner_database_admin::ListDatabaseOperationsRequest, super::spanner_database_admin::ListDatabaseOperationsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/ListDatabaseOperations",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_DATABASE_ADMIN_LIST_BACKUP_OPERATIONS: ::grpcio::Method<super::backup::ListBackupOperationsRequest, super::backup::ListBackupOperationsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.admin.database.v1.DatabaseAdmin/ListBackupOperations",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct DatabaseAdminClient {
    client: ::grpcio::Client,
}

impl DatabaseAdminClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        DatabaseAdminClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn list_databases_opt(&self, req: &super::spanner_database_admin::ListDatabasesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::spanner_database_admin::ListDatabasesResponse> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_LIST_DATABASES, req, opt)
    }

    pub fn list_databases(&self, req: &super::spanner_database_admin::ListDatabasesRequest) -> ::grpcio::Result<super::spanner_database_admin::ListDatabasesResponse> {
        self.list_databases_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_databases_async_opt(&self, req: &super::spanner_database_admin::ListDatabasesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_database_admin::ListDatabasesResponse>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_LIST_DATABASES, req, opt)
    }

    pub fn list_databases_async(&self, req: &super::spanner_database_admin::ListDatabasesRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_database_admin::ListDatabasesResponse>> {
        self.list_databases_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_database_opt(&self, req: &super::spanner_database_admin::CreateDatabaseRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_CREATE_DATABASE, req, opt)
    }

    pub fn create_database(&self, req: &super::spanner_database_admin::CreateDatabaseRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.create_database_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_database_async_opt(&self, req: &super::spanner_database_admin::CreateDatabaseRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_CREATE_DATABASE, req, opt)
    }

    pub fn create_database_async(&self, req: &super::spanner_database_admin::CreateDatabaseRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.create_database_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_database_opt(&self, req: &super::spanner_database_admin::GetDatabaseRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::spanner_database_admin::Database> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_GET_DATABASE, req, opt)
    }

    pub fn get_database(&self, req: &super::spanner_database_admin::GetDatabaseRequest) -> ::grpcio::Result<super::spanner_database_admin::Database> {
        self.get_database_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_database_async_opt(&self, req: &super::spanner_database_admin::GetDatabaseRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_database_admin::Database>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_GET_DATABASE, req, opt)
    }

    pub fn get_database_async(&self, req: &super::spanner_database_admin::GetDatabaseRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_database_admin::Database>> {
        self.get_database_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_database_ddl_opt(&self, req: &super::spanner_database_admin::UpdateDatabaseDdlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_UPDATE_DATABASE_DDL, req, opt)
    }

    pub fn update_database_ddl(&self, req: &super::spanner_database_admin::UpdateDatabaseDdlRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.update_database_ddl_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_database_ddl_async_opt(&self, req: &super::spanner_database_admin::UpdateDatabaseDdlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_UPDATE_DATABASE_DDL, req, opt)
    }

    pub fn update_database_ddl_async(&self, req: &super::spanner_database_admin::UpdateDatabaseDdlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.update_database_ddl_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn drop_database_opt(&self, req: &super::spanner_database_admin::DropDatabaseRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_DROP_DATABASE, req, opt)
    }

    pub fn drop_database(&self, req: &super::spanner_database_admin::DropDatabaseRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.drop_database_opt(req, ::grpcio::CallOption::default())
    }

    pub fn drop_database_async_opt(&self, req: &super::spanner_database_admin::DropDatabaseRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_DROP_DATABASE, req, opt)
    }

    pub fn drop_database_async(&self, req: &super::spanner_database_admin::DropDatabaseRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.drop_database_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_database_ddl_opt(&self, req: &super::spanner_database_admin::GetDatabaseDdlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::spanner_database_admin::GetDatabaseDdlResponse> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_GET_DATABASE_DDL, req, opt)
    }

    pub fn get_database_ddl(&self, req: &super::spanner_database_admin::GetDatabaseDdlRequest) -> ::grpcio::Result<super::spanner_database_admin::GetDatabaseDdlResponse> {
        self.get_database_ddl_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_database_ddl_async_opt(&self, req: &super::spanner_database_admin::GetDatabaseDdlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_database_admin::GetDatabaseDdlResponse>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_GET_DATABASE_DDL, req, opt)
    }

    pub fn get_database_ddl_async(&self, req: &super::spanner_database_admin::GetDatabaseDdlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_database_admin::GetDatabaseDdlResponse>> {
        self.get_database_ddl_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_iam_policy_opt(&self, req: &super::iam_policy::SetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::policy::Policy> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_SET_IAM_POLICY, req, opt)
    }

    pub fn set_iam_policy(&self, req: &super::iam_policy::SetIamPolicyRequest) -> ::grpcio::Result<super::policy::Policy> {
        self.set_iam_policy_opt(req, ::grpcio::CallOption::default())
    }

    pub fn set_iam_policy_async_opt(&self, req: &super::iam_policy::SetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_SET_IAM_POLICY, req, opt)
    }

    pub fn set_iam_policy_async(&self, req: &super::iam_policy::SetIamPolicyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.set_iam_policy_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_iam_policy_opt(&self, req: &super::iam_policy::GetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::policy::Policy> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_GET_IAM_POLICY, req, opt)
    }

    pub fn get_iam_policy(&self, req: &super::iam_policy::GetIamPolicyRequest) -> ::grpcio::Result<super::policy::Policy> {
        self.get_iam_policy_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_iam_policy_async_opt(&self, req: &super::iam_policy::GetIamPolicyRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_GET_IAM_POLICY, req, opt)
    }

    pub fn get_iam_policy_async(&self, req: &super::iam_policy::GetIamPolicyRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::policy::Policy>> {
        self.get_iam_policy_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn test_iam_permissions_opt(&self, req: &super::iam_policy::TestIamPermissionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::iam_policy::TestIamPermissionsResponse> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_TEST_IAM_PERMISSIONS, req, opt)
    }

    pub fn test_iam_permissions(&self, req: &super::iam_policy::TestIamPermissionsRequest) -> ::grpcio::Result<super::iam_policy::TestIamPermissionsResponse> {
        self.test_iam_permissions_opt(req, ::grpcio::CallOption::default())
    }

    pub fn test_iam_permissions_async_opt(&self, req: &super::iam_policy::TestIamPermissionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::iam_policy::TestIamPermissionsResponse>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_TEST_IAM_PERMISSIONS, req, opt)
    }

    pub fn test_iam_permissions_async(&self, req: &super::iam_policy::TestIamPermissionsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::iam_policy::TestIamPermissionsResponse>> {
        self.test_iam_permissions_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_backup_opt(&self, req: &super::backup::CreateBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_CREATE_BACKUP, req, opt)
    }

    pub fn create_backup(&self, req: &super::backup::CreateBackupRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.create_backup_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_backup_async_opt(&self, req: &super::backup::CreateBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_CREATE_BACKUP, req, opt)
    }

    pub fn create_backup_async(&self, req: &super::backup::CreateBackupRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.create_backup_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_backup_opt(&self, req: &super::backup::GetBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::backup::Backup> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_GET_BACKUP, req, opt)
    }

    pub fn get_backup(&self, req: &super::backup::GetBackupRequest) -> ::grpcio::Result<super::backup::Backup> {
        self.get_backup_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_backup_async_opt(&self, req: &super::backup::GetBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::backup::Backup>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_GET_BACKUP, req, opt)
    }

    pub fn get_backup_async(&self, req: &super::backup::GetBackupRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::backup::Backup>> {
        self.get_backup_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_backup_opt(&self, req: &super::backup::UpdateBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::backup::Backup> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_UPDATE_BACKUP, req, opt)
    }

    pub fn update_backup(&self, req: &super::backup::UpdateBackupRequest) -> ::grpcio::Result<super::backup::Backup> {
        self.update_backup_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_backup_async_opt(&self, req: &super::backup::UpdateBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::backup::Backup>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_UPDATE_BACKUP, req, opt)
    }

    pub fn update_backup_async(&self, req: &super::backup::UpdateBackupRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::backup::Backup>> {
        self.update_backup_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_backup_opt(&self, req: &super::backup::DeleteBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_DELETE_BACKUP, req, opt)
    }

    pub fn delete_backup(&self, req: &super::backup::DeleteBackupRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_backup_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_backup_async_opt(&self, req: &super::backup::DeleteBackupRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_DELETE_BACKUP, req, opt)
    }

    pub fn delete_backup_async(&self, req: &super::backup::DeleteBackupRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_backup_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_backups_opt(&self, req: &super::backup::ListBackupsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::backup::ListBackupsResponse> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_LIST_BACKUPS, req, opt)
    }

    pub fn list_backups(&self, req: &super::backup::ListBackupsRequest) -> ::grpcio::Result<super::backup::ListBackupsResponse> {
        self.list_backups_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_backups_async_opt(&self, req: &super::backup::ListBackupsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::backup::ListBackupsResponse>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_LIST_BACKUPS, req, opt)
    }

    pub fn list_backups_async(&self, req: &super::backup::ListBackupsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::backup::ListBackupsResponse>> {
        self.list_backups_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn restore_database_opt(&self, req: &super::spanner_database_admin::RestoreDatabaseRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_RESTORE_DATABASE, req, opt)
    }

    pub fn restore_database(&self, req: &super::spanner_database_admin::RestoreDatabaseRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.restore_database_opt(req, ::grpcio::CallOption::default())
    }

    pub fn restore_database_async_opt(&self, req: &super::spanner_database_admin::RestoreDatabaseRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_RESTORE_DATABASE, req, opt)
    }

    pub fn restore_database_async(&self, req: &super::spanner_database_admin::RestoreDatabaseRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.restore_database_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_database_operations_opt(&self, req: &super::spanner_database_admin::ListDatabaseOperationsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::spanner_database_admin::ListDatabaseOperationsResponse> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_LIST_DATABASE_OPERATIONS, req, opt)
    }

    pub fn list_database_operations(&self, req: &super::spanner_database_admin::ListDatabaseOperationsRequest) -> ::grpcio::Result<super::spanner_database_admin::ListDatabaseOperationsResponse> {
        self.list_database_operations_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_database_operations_async_opt(&self, req: &super::spanner_database_admin::ListDatabaseOperationsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_database_admin::ListDatabaseOperationsResponse>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_LIST_DATABASE_OPERATIONS, req, opt)
    }

    pub fn list_database_operations_async(&self, req: &super::spanner_database_admin::ListDatabaseOperationsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner_database_admin::ListDatabaseOperationsResponse>> {
        self.list_database_operations_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_backup_operations_opt(&self, req: &super::backup::ListBackupOperationsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::backup::ListBackupOperationsResponse> {
        self.client.unary_call(&METHOD_DATABASE_ADMIN_LIST_BACKUP_OPERATIONS, req, opt)
    }

    pub fn list_backup_operations(&self, req: &super::backup::ListBackupOperationsRequest) -> ::grpcio::Result<super::backup::ListBackupOperationsResponse> {
        self.list_backup_operations_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_backup_operations_async_opt(&self, req: &super::backup::ListBackupOperationsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::backup::ListBackupOperationsResponse>> {
        self.client.unary_call_async(&METHOD_DATABASE_ADMIN_LIST_BACKUP_OPERATIONS, req, opt)
    }

    pub fn list_backup_operations_async(&self, req: &super::backup::ListBackupOperationsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::backup::ListBackupOperationsResponse>> {
        self.list_backup_operations_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Output = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait DatabaseAdmin {
    fn list_databases(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_database_admin::ListDatabasesRequest, sink: ::grpcio::UnarySink<super::spanner_database_admin::ListDatabasesResponse>);
    fn create_database(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_database_admin::CreateDatabaseRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn get_database(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_database_admin::GetDatabaseRequest, sink: ::grpcio::UnarySink<super::spanner_database_admin::Database>);
    fn update_database_ddl(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_database_admin::UpdateDatabaseDdlRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn drop_database(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_database_admin::DropDatabaseRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn get_database_ddl(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_database_admin::GetDatabaseDdlRequest, sink: ::grpcio::UnarySink<super::spanner_database_admin::GetDatabaseDdlResponse>);
    fn set_iam_policy(&mut self, ctx: ::grpcio::RpcContext, req: super::iam_policy::SetIamPolicyRequest, sink: ::grpcio::UnarySink<super::policy::Policy>);
    fn get_iam_policy(&mut self, ctx: ::grpcio::RpcContext, req: super::iam_policy::GetIamPolicyRequest, sink: ::grpcio::UnarySink<super::policy::Policy>);
    fn test_iam_permissions(&mut self, ctx: ::grpcio::RpcContext, req: super::iam_policy::TestIamPermissionsRequest, sink: ::grpcio::UnarySink<super::iam_policy::TestIamPermissionsResponse>);
    fn create_backup(&mut self, ctx: ::grpcio::RpcContext, req: super::backup::CreateBackupRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn get_backup(&mut self, ctx: ::grpcio::RpcContext, req: super::backup::GetBackupRequest, sink: ::grpcio::UnarySink<super::backup::Backup>);
    fn update_backup(&mut self, ctx: ::grpcio::RpcContext, req: super::backup::UpdateBackupRequest, sink: ::grpcio::UnarySink<super::backup::Backup>);
    fn delete_backup(&mut self, ctx: ::grpcio::RpcContext, req: super::backup::DeleteBackupRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn list_backups(&mut self, ctx: ::grpcio::RpcContext, req: super::backup::ListBackupsRequest, sink: ::grpcio::UnarySink<super::backup::ListBackupsResponse>);
    fn restore_database(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_database_admin::RestoreDatabaseRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
    fn list_database_operations(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner_database_admin::ListDatabaseOperationsRequest, sink: ::grpcio::UnarySink<super::spanner_database_admin::ListDatabaseOperationsResponse>);
    fn list_backup_operations(&mut self, ctx: ::grpcio::RpcContext, req: super::backup::ListBackupOperationsRequest, sink: ::grpcio::UnarySink<super::backup::ListBackupOperationsResponse>);
}

pub fn create_database_admin<S: DatabaseAdmin + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_LIST_DATABASES, move |ctx, req, resp| {
        instance.list_databases(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_CREATE_DATABASE, move |ctx, req, resp| {
        instance.create_database(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_GET_DATABASE, move |ctx, req, resp| {
        instance.get_database(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_UPDATE_DATABASE_DDL, move |ctx, req, resp| {
        instance.update_database_ddl(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_DROP_DATABASE, move |ctx, req, resp| {
        instance.drop_database(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_GET_DATABASE_DDL, move |ctx, req, resp| {
        instance.get_database_ddl(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_SET_IAM_POLICY, move |ctx, req, resp| {
        instance.set_iam_policy(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_GET_IAM_POLICY, move |ctx, req, resp| {
        instance.get_iam_policy(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_TEST_IAM_PERMISSIONS, move |ctx, req, resp| {
        instance.test_iam_permissions(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_CREATE_BACKUP, move |ctx, req, resp| {
        instance.create_backup(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_GET_BACKUP, move |ctx, req, resp| {
        instance.get_backup(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_UPDATE_BACKUP, move |ctx, req, resp| {
        instance.update_backup(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_DELETE_BACKUP, move |ctx, req, resp| {
        instance.delete_backup(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_LIST_BACKUPS, move |ctx, req, resp| {
        instance.list_backups(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_RESTORE_DATABASE, move |ctx, req, resp| {
        instance.restore_database(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_LIST_DATABASE_OPERATIONS, move |ctx, req, resp| {
        instance.list_database_operations(ctx, req, resp)
    });
    let mut instance = s;
    builder = builder.add_unary_handler(&METHOD_DATABASE_ADMIN_LIST_BACKUP_OPERATIONS, move |ctx, req, resp| {
        instance.list_backup_operations(ctx, req, resp)
    });
    builder.build()
}
