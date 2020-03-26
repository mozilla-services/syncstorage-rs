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

const METHOD_BIGTABLE_CLUSTER_SERVICE_LIST_ZONES: ::grpcio::Method<super::bigtable_cluster_service_messages::ListZonesRequest, super::bigtable_cluster_service_messages::ListZonesResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.cluster.v1.BigtableClusterService/ListZones",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_CLUSTER_SERVICE_GET_CLUSTER: ::grpcio::Method<super::bigtable_cluster_service_messages::GetClusterRequest, super::bigtable_cluster_data::Cluster> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.cluster.v1.BigtableClusterService/GetCluster",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_CLUSTER_SERVICE_LIST_CLUSTERS: ::grpcio::Method<super::bigtable_cluster_service_messages::ListClustersRequest, super::bigtable_cluster_service_messages::ListClustersResponse> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.cluster.v1.BigtableClusterService/ListClusters",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_CLUSTER_SERVICE_CREATE_CLUSTER: ::grpcio::Method<super::bigtable_cluster_service_messages::CreateClusterRequest, super::bigtable_cluster_data::Cluster> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.cluster.v1.BigtableClusterService/CreateCluster",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_CLUSTER_SERVICE_UPDATE_CLUSTER: ::grpcio::Method<super::bigtable_cluster_data::Cluster, super::bigtable_cluster_data::Cluster> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.cluster.v1.BigtableClusterService/UpdateCluster",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_CLUSTER_SERVICE_DELETE_CLUSTER: ::grpcio::Method<super::bigtable_cluster_service_messages::DeleteClusterRequest, super::empty::Empty> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.cluster.v1.BigtableClusterService/DeleteCluster",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

const METHOD_BIGTABLE_CLUSTER_SERVICE_UNDELETE_CLUSTER: ::grpcio::Method<super::bigtable_cluster_service_messages::UndeleteClusterRequest, super::operations::Operation> = ::grpcio::Method {
    ty: ::grpcio::MethodType::Unary,
    name: "/google.bigtable.admin.cluster.v1.BigtableClusterService/UndeleteCluster",
    req_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
    resp_mar: ::grpcio::Marshaller { ser: ::grpcio::pb_ser, de: ::grpcio::pb_de },
};

#[derive(Clone)]
pub struct BigtableClusterServiceClient {
    client: ::grpcio::Client,
}

impl BigtableClusterServiceClient {
    pub fn new(channel: ::grpcio::Channel) -> Self {
        BigtableClusterServiceClient {
            client: ::grpcio::Client::new(channel),
        }
    }

    pub fn list_zones_opt(&self, req: &super::bigtable_cluster_service_messages::ListZonesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_cluster_service_messages::ListZonesResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_CLUSTER_SERVICE_LIST_ZONES, req, opt)
    }

    pub fn list_zones(&self, req: &super::bigtable_cluster_service_messages::ListZonesRequest) -> ::grpcio::Result<super::bigtable_cluster_service_messages::ListZonesResponse> {
        self.list_zones_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_zones_async_opt(&self, req: &super::bigtable_cluster_service_messages::ListZonesRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_cluster_service_messages::ListZonesResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_CLUSTER_SERVICE_LIST_ZONES, req, opt)
    }

    pub fn list_zones_async(&self, req: &super::bigtable_cluster_service_messages::ListZonesRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_cluster_service_messages::ListZonesResponse>> {
        self.list_zones_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_cluster_opt(&self, req: &super::bigtable_cluster_service_messages::GetClusterRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_cluster_data::Cluster> {
        self.client.unary_call(&METHOD_BIGTABLE_CLUSTER_SERVICE_GET_CLUSTER, req, opt)
    }

    pub fn get_cluster(&self, req: &super::bigtable_cluster_service_messages::GetClusterRequest) -> ::grpcio::Result<super::bigtable_cluster_data::Cluster> {
        self.get_cluster_opt(req, ::grpcio::CallOption::default())
    }

    pub fn get_cluster_async_opt(&self, req: &super::bigtable_cluster_service_messages::GetClusterRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_cluster_data::Cluster>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_CLUSTER_SERVICE_GET_CLUSTER, req, opt)
    }

    pub fn get_cluster_async(&self, req: &super::bigtable_cluster_service_messages::GetClusterRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_cluster_data::Cluster>> {
        self.get_cluster_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_clusters_opt(&self, req: &super::bigtable_cluster_service_messages::ListClustersRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_cluster_service_messages::ListClustersResponse> {
        self.client.unary_call(&METHOD_BIGTABLE_CLUSTER_SERVICE_LIST_CLUSTERS, req, opt)
    }

    pub fn list_clusters(&self, req: &super::bigtable_cluster_service_messages::ListClustersRequest) -> ::grpcio::Result<super::bigtable_cluster_service_messages::ListClustersResponse> {
        self.list_clusters_opt(req, ::grpcio::CallOption::default())
    }

    pub fn list_clusters_async_opt(&self, req: &super::bigtable_cluster_service_messages::ListClustersRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_cluster_service_messages::ListClustersResponse>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_CLUSTER_SERVICE_LIST_CLUSTERS, req, opt)
    }

    pub fn list_clusters_async(&self, req: &super::bigtable_cluster_service_messages::ListClustersRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_cluster_service_messages::ListClustersResponse>> {
        self.list_clusters_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_cluster_opt(&self, req: &super::bigtable_cluster_service_messages::CreateClusterRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_cluster_data::Cluster> {
        self.client.unary_call(&METHOD_BIGTABLE_CLUSTER_SERVICE_CREATE_CLUSTER, req, opt)
    }

    pub fn create_cluster(&self, req: &super::bigtable_cluster_service_messages::CreateClusterRequest) -> ::grpcio::Result<super::bigtable_cluster_data::Cluster> {
        self.create_cluster_opt(req, ::grpcio::CallOption::default())
    }

    pub fn create_cluster_async_opt(&self, req: &super::bigtable_cluster_service_messages::CreateClusterRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_cluster_data::Cluster>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_CLUSTER_SERVICE_CREATE_CLUSTER, req, opt)
    }

    pub fn create_cluster_async(&self, req: &super::bigtable_cluster_service_messages::CreateClusterRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_cluster_data::Cluster>> {
        self.create_cluster_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_cluster_opt(&self, req: &super::bigtable_cluster_data::Cluster, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::bigtable_cluster_data::Cluster> {
        self.client.unary_call(&METHOD_BIGTABLE_CLUSTER_SERVICE_UPDATE_CLUSTER, req, opt)
    }

    pub fn update_cluster(&self, req: &super::bigtable_cluster_data::Cluster) -> ::grpcio::Result<super::bigtable_cluster_data::Cluster> {
        self.update_cluster_opt(req, ::grpcio::CallOption::default())
    }

    pub fn update_cluster_async_opt(&self, req: &super::bigtable_cluster_data::Cluster, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_cluster_data::Cluster>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_CLUSTER_SERVICE_UPDATE_CLUSTER, req, opt)
    }

    pub fn update_cluster_async(&self, req: &super::bigtable_cluster_data::Cluster) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::bigtable_cluster_data::Cluster>> {
        self.update_cluster_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_cluster_opt(&self, req: &super::bigtable_cluster_service_messages::DeleteClusterRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::empty::Empty> {
        self.client.unary_call(&METHOD_BIGTABLE_CLUSTER_SERVICE_DELETE_CLUSTER, req, opt)
    }

    pub fn delete_cluster(&self, req: &super::bigtable_cluster_service_messages::DeleteClusterRequest) -> ::grpcio::Result<super::empty::Empty> {
        self.delete_cluster_opt(req, ::grpcio::CallOption::default())
    }

    pub fn delete_cluster_async_opt(&self, req: &super::bigtable_cluster_service_messages::DeleteClusterRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_CLUSTER_SERVICE_DELETE_CLUSTER, req, opt)
    }

    pub fn delete_cluster_async(&self, req: &super::bigtable_cluster_service_messages::DeleteClusterRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::empty::Empty>> {
        self.delete_cluster_async_opt(req, ::grpcio::CallOption::default())
    }

    pub fn undelete_cluster_opt(&self, req: &super::bigtable_cluster_service_messages::UndeleteClusterRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<super::operations::Operation> {
        self.client.unary_call(&METHOD_BIGTABLE_CLUSTER_SERVICE_UNDELETE_CLUSTER, req, opt)
    }

    pub fn undelete_cluster(&self, req: &super::bigtable_cluster_service_messages::UndeleteClusterRequest) -> ::grpcio::Result<super::operations::Operation> {
        self.undelete_cluster_opt(req, ::grpcio::CallOption::default())
    }

    pub fn undelete_cluster_async_opt(&self, req: &super::bigtable_cluster_service_messages::UndeleteClusterRequest, opt: ::grpcio::CallOption) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.client.unary_call_async(&METHOD_BIGTABLE_CLUSTER_SERVICE_UNDELETE_CLUSTER, req, opt)
    }

    pub fn undelete_cluster_async(&self, req: &super::bigtable_cluster_service_messages::UndeleteClusterRequest) -> ::grpcio::Result<::grpcio::ClientUnaryReceiver<super::operations::Operation>> {
        self.undelete_cluster_async_opt(req, ::grpcio::CallOption::default())
    }
    pub fn spawn<F>(&self, f: F) where F: ::futures::Future<Item = (), Error = ()> + Send + 'static {
        self.client.spawn(f)
    }
}

pub trait BigtableClusterService {
    fn list_zones(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_cluster_service_messages::ListZonesRequest, sink: ::grpcio::UnarySink<super::bigtable_cluster_service_messages::ListZonesResponse>);
    fn get_cluster(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_cluster_service_messages::GetClusterRequest, sink: ::grpcio::UnarySink<super::bigtable_cluster_data::Cluster>);
    fn list_clusters(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_cluster_service_messages::ListClustersRequest, sink: ::grpcio::UnarySink<super::bigtable_cluster_service_messages::ListClustersResponse>);
    fn create_cluster(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_cluster_service_messages::CreateClusterRequest, sink: ::grpcio::UnarySink<super::bigtable_cluster_data::Cluster>);
    fn update_cluster(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_cluster_data::Cluster, sink: ::grpcio::UnarySink<super::bigtable_cluster_data::Cluster>);
    fn delete_cluster(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_cluster_service_messages::DeleteClusterRequest, sink: ::grpcio::UnarySink<super::empty::Empty>);
    fn undelete_cluster(&mut self, ctx: ::grpcio::RpcContext, req: super::bigtable_cluster_service_messages::UndeleteClusterRequest, sink: ::grpcio::UnarySink<super::operations::Operation>);
}

pub fn create_bigtable_cluster_service<S: BigtableClusterService + Send + Clone + 'static>(s: S) -> ::grpcio::Service {
    let mut builder = ::grpcio::ServiceBuilder::new();
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_CLUSTER_SERVICE_LIST_ZONES, move |ctx, req, resp| {
        instance.list_zones(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_CLUSTER_SERVICE_GET_CLUSTER, move |ctx, req, resp| {
        instance.get_cluster(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_CLUSTER_SERVICE_LIST_CLUSTERS, move |ctx, req, resp| {
        instance.list_clusters(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_CLUSTER_SERVICE_CREATE_CLUSTER, move |ctx, req, resp| {
        instance.create_cluster(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_CLUSTER_SERVICE_UPDATE_CLUSTER, move |ctx, req, resp| {
        instance.update_cluster(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_CLUSTER_SERVICE_DELETE_CLUSTER, move |ctx, req, resp| {
        instance.delete_cluster(ctx, req, resp)
    });
    let mut instance = s.clone();
    builder = builder.add_unary_handler(&METHOD_BIGTABLE_CLUSTER_SERVICE_UNDELETE_CLUSTER, move |ctx, req, resp| {
        instance.undelete_cluster(ctx, req, resp)
    });
    builder.build()
}
