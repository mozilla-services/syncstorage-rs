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

const METHOD_SPANNER_CREATE_SESSION: ::grpcio::Method<super::spanner::CreateSessionRequest, super::spanner::Session> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.v1.Spanner/CreateSession",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SPANNER_GET_SESSION: ::grpcio::Method<super::spanner::GetSessionRequest, super::spanner::Session> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.v1.Spanner/GetSession",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SPANNER_LIST_SESSIONS: ::grpcio::Method<super::spanner::ListSessionsRequest, super::spanner::ListSessionsResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.v1.Spanner/ListSessions",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SPANNER_DELETE_SESSION: ::grpcio::Method<super::spanner::DeleteSessionRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.v1.Spanner/DeleteSession",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SPANNER_EXECUTE_SQL: ::grpcio::Method<super::spanner::ExecuteSqlRequest, super::result_set::ResultSet> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.v1.Spanner/ExecuteSql",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SPANNER_EXECUTE_STREAMING_SQL: ::grpcio::Method<super::spanner::ExecuteSqlRequest, super::result_set::PartialResultSet> = ::grpcio::Method {
    ty: ::grpcio::MethodType::ServerStreaming,
    name: "/google.spanner.v1.Spanner/ExecuteStreamingSql",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SPANNER_READ: ::grpcio::Method<super::spanner::ReadRequest, super::result_set::ResultSet> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.v1.Spanner/Read",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SPANNER_STREAMING_READ: ::grpcio::Method<super::spanner::ReadRequest, super::result_set::PartialResultSet> = ::grpcio::Method {
    ty: ::grpcio::MethodType::ServerStreaming,
    name: "/google.spanner.v1.Spanner/StreamingRead",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SPANNER_BEGIN_TRANSACTION: ::grpcio::Method<super::spanner::BeginTransactionRequest, super::transaction::Transaction> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.v1.Spanner/BeginTransaction",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SPANNER_COMMIT: ::grpcio::Method<super::spanner::CommitRequest, super::spanner::CommitResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.v1.Spanner/Commit",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SPANNER_ROLLBACK: ::grpcio::Method<super::spanner::RollbackRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.v1.Spanner/Rollback",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SPANNER_PARTITION_QUERY: ::grpcio::Method<super::spanner::PartitionQueryRequest, super::spanner::PartitionResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.v1.Spanner/PartitionQuery",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SPANNER_PARTITION_READ: ::grpcio::Method<super::spanner::PartitionReadRequest, super::spanner::PartitionResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.spanner.v1.Spanner/PartitionRead",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct SpannerClient {
    client: ::grpcio::Client,
}

impl SpannerClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        SpannerClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn create_session_opt(&self, req: &super::spanner::CreateSessionRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::spanner::Session> {
        self.client.unary_call(&METHOD_SPANNER_CREATE_SESSION, req, opt)
    }

    pub fn create_session(&self, req: &super::spanner::CreateSessionRequest) -> ::grpcio::Result<super::spanner::Session> {
        self.create_session_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_session_async_opt(&self, req: &super::spanner::CreateSessionRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner::Session>> {
        self.client.unary_call_async(&METHOD_SPANNER_CREATE_SESSION, req, opt)
    }

    pub fn create_session_async(&self, req: &super::spanner::CreateSessionRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner::Session>> {
        self.create_session_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_session_opt(&self, req: &super::spanner::GetSessionRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::spanner::Session> {
        self.client.unary_call(&METHOD_SPANNER_GET_SESSION, req, opt)
    }

    pub fn get_session(&self, req: &super::spanner::GetSessionRequest) -> ::grpcio::Result<super::spanner::Session> {
        self.get_session_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_session_async_opt(&self, req: &super::spanner::GetSessionRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner::Session>> {
        self.client.unary_call_async(&METHOD_SPANNER_GET_SESSION, req, opt)
    }

    pub fn get_session_async(&self, req: &super::spanner::GetSessionRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner::Session>> {
        self.get_session_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_sessions_opt(&self, req: &super::spanner::ListSessionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::spanner::ListSessionsResponse> {
        self.client.unary_call(&METHOD_SPANNER_LIST_SESSIONS, req, opt)
    }

    pub fn list_sessions(&self, req: &super::spanner::ListSessionsRequest) -> ::grpcio::Result<super::spanner::ListSessionsResponse> {
        self.list_sessions_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_sessions_async_opt(&self, req: &super::spanner::ListSessionsRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner::ListSessionsResponse>> {
        self.client.unary_call_async(&METHOD_SPANNER_LIST_SESSIONS, req, opt)
    }

    pub fn list_sessions_async(&self, req: &super::spanner::ListSessionsRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner::ListSessionsResponse>> {
        self.list_sessions_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_session_opt(&self, req: &super::spanner::DeleteSessionRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_SPANNER_DELETE_SESSION, req, opt)
    }

    pub fn delete_session(&self, req: &super::spanner::DeleteSessionRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_session_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_session_async_opt(&self, req: &super::spanner::DeleteSessionRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_SPANNER_DELETE_SESSION, req, opt)
    }

    pub fn delete_session_async(&self, req: &super::spanner::DeleteSessionRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_session_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn execute_sql_opt(&self, req: &super::spanner::ExecuteSqlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::result_set::ResultSet> {
        self.client.unary_call(&METHOD_SPANNER_EXECUTE_SQL, req, opt)
    }

    pub fn execute_sql(&self, req: &super::spanner::ExecuteSqlRequest) -> ::grpcio::Result<super::result_set::ResultSet> {
        self.execute_sql_opt(req, ::grpcio::CallOption::default())
    }

    pub fn execute_sql_async_opt(&self, req: &super::spanner::ExecuteSqlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::result_set::ResultSet>> {
        self.client.unary_call_async(&METHOD_SPANNER_EXECUTE_SQL, req, opt)
    }

    pub fn execute_sql_async(&self, req: &super::spanner::ExecuteSqlRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::result_set::ResultSet>> {
        self.execute_sql_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn execute_streaming_sql_opt(&self, req: &super::spanner::ExecuteSqlRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::result_set::PartialResultSet>> {
        self.client.server_streaming(&METHOD_SPANNER_EXECUTE_STREAMING_SQL, req, opt)
    }

    pub fn execute_streaming_sql(&self, req: &super::spanner::ExecuteSqlRequest) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::result_set::PartialResultSet>> {
        self.execute_streaming_sql_opt(req, ::grpcio::CallOption::default())
    }

    pub fn read_opt(&self, req: &super::spanner::ReadRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::result_set::ResultSet> {
        self.client.unary_call(&METHOD_SPANNER_READ, req, opt)
    }

    pub fn read(&self, req: &super::spanner::ReadRequest) -> ::grpcio::Result<super::result_set::ResultSet> {
        self.read_opt(req, ::grpcio::CallOption::default())
    }

    pub fn read_async_opt(&self, req: &super::spanner::ReadRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::result_set::ResultSet>> {
        self.client.unary_call_async(&METHOD_SPANNER_READ, req, opt)
    }

    pub fn read_async(&self, req: &super::spanner::ReadRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::result_set::ResultSet>> {
        self.read_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn streaming_read_opt(&self, req: &super::spanner::ReadRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::result_set::PartialResultSet>> {
        self.client.server_streaming(&METHOD_SPANNER_STREAMING_READ, req, opt)
    }

    pub fn streaming_read(&self, req: &super::spanner::ReadRequest) -> ::grpcio::Result<::grpcio::ClientSStreamReceiver<super::result_set::PartialResultSet>> {
        self.streaming_read_opt(req, ::grpcio::CallOption::default())
    }

    pub fn begin_transaction_opt(&self, req: &super::spanner::BeginTransactionRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::transaction::Transaction> {
        self.client.unary_call(&METHOD_SPANNER_BEGIN_TRANSACTION, req, opt)
    }

    pub fn begin_transaction(&self, req: &super::spanner::BeginTransactionRequest) -> ::grpcio::Result<super::transaction::Transaction> {
        self.begin_transaction_opt(req, ::grpcio::CallOption::default())
    }

    pub fn begin_transaction_async_opt(&self, req: &super::spanner::BeginTransactionRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::transaction::Transaction>> {
        self.client.unary_call_async(&METHOD_SPANNER_BEGIN_TRANSACTION, req, opt)
    }

    pub fn begin_transaction_async(&self, req: &super::spanner::BeginTransactionRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::transaction::Transaction>> {
        self.begin_transaction_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn commit_opt(&self, req: &super::spanner::CommitRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::spanner::CommitResponse> {
        self.client.unary_call(&METHOD_SPANNER_COMMIT, req, opt)
    }

    pub fn commit(&self, req: &super::spanner::CommitRequest) -> ::grpcio::Result<super::spanner::CommitResponse> {
        self.commit_opt(req, ::grpcio::CallOption::default())
    }

    pub fn commit_async_opt(&self, req: &super::spanner::CommitRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner::CommitResponse>> {
        self.client.unary_call_async(&METHOD_SPANNER_COMMIT, req, opt)
    }

    pub fn commit_async(&self, req: &super::spanner::CommitRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner::CommitResponse>> {
        self.commit_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn rollback_opt(&self, req: &super::spanner::RollbackRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_SPANNER_ROLLBACK, req, opt)
    }

    pub fn rollback(&self, req: &super::spanner::RollbackRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.rollback_opt(req, ::grpcio::CallOption::default())
    }

    pub fn rollback_async_opt(&self, req: &super::spanner::RollbackRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_SPANNER_ROLLBACK, req, opt)
    }

    pub fn rollback_async(&self, req: &super::spanner::RollbackRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.rollback_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn partition_query_opt(&self, req: &super::spanner::PartitionQueryRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::spanner::PartitionResponse> {
        self.client.unary_call(&METHOD_SPANNER_PARTITION_QUERY, req, opt)
    }

    pub fn partition_query(&self, req: &super::spanner::PartitionQueryRequest) -> ::grpcio::Result<super::spanner::PartitionResponse> {
        self.partition_query_opt(req, ::grpcio::CallOption::default())
    }

    pub fn partition_query_async_opt(&self, req: &super::spanner::PartitionQueryRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner::PartitionResponse>> {
        self.client.unary_call_async(&METHOD_SPANNER_PARTITION_QUERY, req, opt)
    }

    pub fn partition_query_async(&self, req: &super::spanner::PartitionQueryRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner::PartitionResponse>> {
        self.partition_query_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn partition_read_opt(&self, req: &super::spanner::PartitionReadRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::spanner::PartitionResponse> {
        self.client.unary_call(&METHOD_SPANNER_PARTITION_READ, req, opt)
    }

    pub fn partition_read(&self, req: &super::spanner::PartitionReadRequest) -> ::grpcio::Result<super::spanner::PartitionResponse> {
        self.partition_read_opt(req, ::grpcio::CallOption::default())
    }

    pub fn partition_read_async_opt(&self, req: &super::spanner::PartitionReadRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner::PartitionResponse>> {
        self.client.unary_call_async(&METHOD_SPANNER_PARTITION_READ, req, opt)
    }

    pub fn partition_read_async(&self, req: &super::spanner::PartitionReadRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::spanner::PartitionResponse>> {
        self.partition_read_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait Spanner {
    fn create_session(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner::CreateSessionRequest, sink: ::grpcio::UnarySink<super::spanner::Session>);
    fn get_session(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner::GetSessionRequest, sink: ::grpcio::UnarySink<super::spanner::Session>);
    fn list_sessions(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner::ListSessionsRequest, sink: ::grpcio::UnarySink<super::spanner::ListSessionsResponse>);
    fn delete_session(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner::DeleteSessionRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn execute_sql(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner::ExecuteSqlRequest, sink: ::grpcio::UnarySink<super::result_set::ResultSet>);
    fn execute_streaming_sql(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner::ExecuteSqlRequest, sink: ::grpcio::ServerStreamingSink<super::result_set::PartialResultSet>);
    fn read(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner::ReadRequest, sink: ::grpcio::UnarySink<super::result_set::ResultSet>);
    fn streaming_read(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner::ReadRequest, sink: ::grpcio::ServerStreamingSink<super::result_set::PartialResultSet>);
    fn begin_transaction(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner::BeginTransactionRequest, sink: ::grpcio::UnarySink<super::transaction::Transaction>);
    fn commit(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner::CommitRequest, sink: ::grpcio::UnarySink<super::spanner::CommitResponse>);
    fn rollback(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner::RollbackRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn partition_query(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner::PartitionQueryRequest, sink: ::grpcio::UnarySink<super::spanner::PartitionResponse>);
    fn partition_read(&mut self, ctx: ::grpcio::RpcContext, req: super::spanner::PartitionReadRequest, sink: ::grpcio::UnarySink<super::spanner::PartitionResponse>);
}

pub fn create_spanner<S: Spanner + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SPANNER_CREATE_SESSION, move |ctx, req, resp| {
        instance.create_session(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SPANNER_GET_SESSION, move |ctx, req, resp| {
        instance.get_session(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SPANNER_LIST_SESSIONS, move |ctx, req, resp| {
        instance.list_sessions(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SPANNER_DELETE_SESSION, move |ctx, req, resp| {
        instance.delete_session(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SPANNER_EXECUTE_SQL, move |ctx, req, resp| {
        instance.execute_sql(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_server_streaming_handler(&METHOD_SPANNER_EXECUTE_STREAMING_SQL, move |ctx, req, resp| {
        instance.execute_streaming_sql(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SPANNER_READ, move |ctx, req, resp| {
        instance.read(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_server_streaming_handler(&METHOD_SPANNER_STREAMING_READ, move |ctx, req, resp| {
        instance.streaming_read(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SPANNER_BEGIN_TRANSACTION, move |ctx, req, resp| {
        instance.begin_transaction(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SPANNER_COMMIT, move |ctx, req, resp| {
        instance.commit(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SPANNER_ROLLBACK, move |ctx, req, resp| {
        instance.rollback(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SPANNER_PARTITION_QUERY, move |ctx, req, resp| {
        instance.partition_query(ctx, req, resp)
    });
    let mut instance = s;
    builder = builder.add_unary_handler(&METHOD_SPANNER_PARTITION_READ, move |ctx, req, resp| {
        instance.partition_read(ctx, req, resp)
    });
    builder.build()
}
