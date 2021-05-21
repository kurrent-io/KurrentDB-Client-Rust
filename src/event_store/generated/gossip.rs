#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClusterInfo {
    #[prost(message, repeated, tag = "1")]
    pub members: ::prost::alloc::vec::Vec<MemberInfo>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EndPoint {
    #[prost(string, tag = "1")]
    pub address: ::prost::alloc::string::String,
    #[prost(uint32, tag = "2")]
    pub port: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MemberInfo {
    #[prost(message, optional, tag = "1")]
    pub instance_id: ::core::option::Option<super::Uuid>,
    #[prost(int64, tag = "2")]
    pub time_stamp: i64,
    #[prost(enumeration = "member_info::VNodeState", tag = "3")]
    pub state: i32,
    #[prost(bool, tag = "4")]
    pub is_alive: bool,
    #[prost(message, optional, tag = "5")]
    pub http_end_point: ::core::option::Option<EndPoint>,
}
/// Nested message and enum types in `MemberInfo`.
pub mod member_info {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum VNodeState {
        Initializing = 0,
        DiscoverLeader = 1,
        Unknown = 2,
        PreReplica = 3,
        CatchingUp = 4,
        Clone = 5,
        Follower = 6,
        PreLeader = 7,
        Leader = 8,
        Manager = 9,
        ShuttingDown = 10,
        Shutdown = 11,
        ReadOnlyLeaderless = 12,
        PreReadOnlyReplica = 13,
        ReadOnlyReplica = 14,
        ResigningLeader = 15,
    }
}
#[doc = r" Generated client implementations."]
pub mod gossip_client {
    #![allow(unused_variables, dead_code, missing_docs)]
    use tonic::codegen::*;
    pub struct GossipClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl GossipClient<tonic::transport::Channel> {
        #[doc = r" Attempt to create a new client by connecting to a given endpoint."]
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> GossipClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::ResponseBody: Body + HttpBody + Send + 'static,
        T::Error: Into<StdError>,
        <T::ResponseBody as HttpBody>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor(inner: T, interceptor: impl Into<tonic::Interceptor>) -> Self {
            let inner = tonic::client::Grpc::with_interceptor(inner, interceptor);
            Self { inner }
        }
        pub async fn read(
            &mut self,
            request: impl tonic::IntoRequest<super::super::Empty>,
        ) -> Result<tonic::Response<super::ClusterInfo>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path =
                http::uri::PathAndQuery::from_static("/event_store.client.gossip.Gossip/Read");
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
    impl<T: Clone> Clone for GossipClient<T> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
            }
        }
    }
    impl<T> std::fmt::Debug for GossipClient<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "GossipClient {{ ... }}")
        }
    }
}
