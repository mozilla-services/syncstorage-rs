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

const METHOD_SCHEMA_SERVICE_CREATE_SCHEMA: ::grpcio::Method<super::schema::CreateSchemaRequest, super::schema::Schema> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.SchemaService/CreateSchema",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SCHEMA_SERVICE_GET_SCHEMA: ::grpcio::Method<super::schema::GetSchemaRequest, super::schema::Schema> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.SchemaService/GetSchema",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SCHEMA_SERVICE_LIST_SCHEMAS: ::grpcio::Method<super::schema::ListSchemasRequest, super::schema::ListSchemasResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.SchemaService/ListSchemas",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SCHEMA_SERVICE_DELETE_SCHEMA: ::grpcio::Method<super::schema::DeleteSchemaRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.SchemaService/DeleteSchema",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SCHEMA_SERVICE_VALIDATE_SCHEMA: ::grpcio::Method<super::schema::ValidateSchemaRequest, super::schema::ValidateSchemaResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.SchemaService/ValidateSchema",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_SCHEMA_SERVICE_VALIDATE_MESSAGE: ::grpcio::Method<super::schema::ValidateMessageRequest, super::schema::ValidateMessageResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.pubsub.v1.SchemaService/ValidateMessage",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct SchemaServiceClient {
    client: ::grpcio::Client,
}

impl SchemaServiceClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        SchemaServiceClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn create_schema_opt(&self, req: &super::schema::CreateSchemaRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::schema::Schema> {
        self.client.unary_call(&METHOD_SCHEMA_SERVICE_CREATE_SCHEMA, req, opt)
    }

    pub fn create_schema(&self, req: &super::schema::CreateSchemaRequest) -> ::grpcio::Result<super::schema::Schema> {
        self.create_schema_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_schema_async_opt(&self, req: &super::schema::CreateSchemaRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::schema::Schema>> {
        self.client.unary_call_async(&METHOD_SCHEMA_SERVICE_CREATE_SCHEMA, req, opt)
    }

    pub fn create_schema_async(&self, req: &super::schema::CreateSchemaRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::schema::Schema>> {
        self.create_schema_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_schema_opt(&self, req: &super::schema::GetSchemaRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::schema::Schema> {
        self.client.unary_call(&METHOD_SCHEMA_SERVICE_GET_SCHEMA, req, opt)
    }

    pub fn get_schema(&self, req: &super::schema::GetSchemaRequest) -> ::grpcio::Result<super::schema::Schema> {
        self.get_schema_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_schema_async_opt(&self, req: &super::schema::GetSchemaRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::schema::Schema>> {
        self.client.unary_call_async(&METHOD_SCHEMA_SERVICE_GET_SCHEMA, req, opt)
    }

    pub fn get_schema_async(&self, req: &super::schema::GetSchemaRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::schema::Schema>> {
        self.get_schema_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_schemas_opt(&self, req: &super::schema::ListSchemasRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::schema::ListSchemasResponse> {
        self.client.unary_call(&METHOD_SCHEMA_SERVICE_LIST_SCHEMAS, req, opt)
    }

    pub fn list_schemas(&self, req: &super::schema::ListSchemasRequest) -> ::grpcio::Result<super::schema::ListSchemasResponse> {
        self.list_schemas_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_schemas_async_opt(&self, req: &super::schema::ListSchemasRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::schema::ListSchemasResponse>> {
        self.client.unary_call_async(&METHOD_SCHEMA_SERVICE_LIST_SCHEMAS, req, opt)
    }

    pub fn list_schemas_async(&self, req: &super::schema::ListSchemasRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::schema::ListSchemasResponse>> {
        self.list_schemas_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_schema_opt(&self, req: &super::schema::DeleteSchemaRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_SCHEMA_SERVICE_DELETE_SCHEMA, req, opt)
    }

    pub fn delete_schema(&self, req: &super::schema::DeleteSchemaRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_schema_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_schema_async_opt(&self, req: &super::schema::DeleteSchemaRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_SCHEMA_SERVICE_DELETE_SCHEMA, req, opt)
    }

    pub fn delete_schema_async(&self, req: &super::schema::DeleteSchemaRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_schema_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn validate_schema_opt(&self, req: &super::schema::ValidateSchemaRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::schema::ValidateSchemaResponse> {
        self.client.unary_call(&METHOD_SCHEMA_SERVICE_VALIDATE_SCHEMA, req, opt)
    }

    pub fn validate_schema(&self, req: &super::schema::ValidateSchemaRequest) -> ::grpcio::Result<super::schema::ValidateSchemaResponse> {
        self.validate_schema_opt(req, ::grpcio::CallOption::default())
    }

    pub fn validate_schema_async_opt(&self, req: &super::schema::ValidateSchemaRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::schema::ValidateSchemaResponse>> {
        self.client.unary_call_async(&METHOD_SCHEMA_SERVICE_VALIDATE_SCHEMA, req, opt)
    }

    pub fn validate_schema_async(&self, req: &super::schema::ValidateSchemaRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::schema::ValidateSchemaResponse>> {
        self.validate_schema_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn validate_message_opt(&self, req: &super::schema::ValidateMessageRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::schema::ValidateMessageResponse> {
        self.client.unary_call(&METHOD_SCHEMA_SERVICE_VALIDATE_MESSAGE, req, opt)
    }

    pub fn validate_message(&self, req: &super::schema::ValidateMessageRequest) -> ::grpcio::Result<super::schema::ValidateMessageResponse> {
        self.validate_message_opt(req, ::grpcio::CallOption::default())
    }

    pub fn validate_message_async_opt(&self, req: &super::schema::ValidateMessageRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::schema::ValidateMessageResponse>> {
        self.client.unary_call_async(&METHOD_SCHEMA_SERVICE_VALIDATE_MESSAGE, req, opt)
    }

    pub fn validate_message_async(&self, req: &super::schema::ValidateMessageRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::schema::ValidateMessageResponse>> {
        self.validate_message_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Output = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait SchemaService {
    fn create_schema(&mut self, ctx: ::grpcio::RpcContext, req: super::schema::CreateSchemaRequest, sink: ::grpcio::UnarySink<super::schema::Schema>);
    fn get_schema(&mut self, ctx: ::grpcio::RpcContext, req: super::schema::GetSchemaRequest, sink: ::grpcio::UnarySink<super::schema::Schema>);
    fn list_schemas(&mut self, ctx: ::grpcio::RpcContext, req: super::schema::ListSchemasRequest, sink: ::grpcio::UnarySink<super::schema::ListSchemasResponse>);
    fn delete_schema(&mut self, ctx: ::grpcio::RpcContext, req: super::schema::DeleteSchemaRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn validate_schema(&mut self, ctx: ::grpcio::RpcContext, req: super::schema::ValidateSchemaRequest, sink: ::grpcio::UnarySink<super::schema::ValidateSchemaResponse>);
    fn validate_message(&mut self, ctx: ::grpcio::RpcContext, req: super::schema::ValidateMessageRequest, sink: ::grpcio::UnarySink<super::schema::ValidateMessageResponse>);
}

pub fn create_schema_service<S: SchemaService + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SCHEMA_SERVICE_CREATE_SCHEMA, move |ctx, req, resp| {
        instance.create_schema(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SCHEMA_SERVICE_GET_SCHEMA, move |ctx, req, resp| {
        instance.get_schema(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SCHEMA_SERVICE_LIST_SCHEMAS, move |ctx, req, resp| {
        instance.list_schemas(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SCHEMA_SERVICE_DELETE_SCHEMA, move |ctx, req, resp| {
        instance.delete_schema(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_SCHEMA_SERVICE_VALIDATE_SCHEMA, move |ctx, req, resp| {
        instance.validate_schema(ctx, req, resp)
    });
    let mut instance = s;
    builder = builder.add_unary_handler(&METHOD_SCHEMA_SERVICE_VALIDATE_MESSAGE, move |ctx, req, resp| {
        instance.validate_message(ctx, req, resp)
    });
    builder.build()
}
