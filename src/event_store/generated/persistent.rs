#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReadReq {
    #[prost(oneof = "read_req::Content", tags = "1, 2, 3")]
    pub content: ::core::option::Option<read_req::Content>,
}
/// Nested message and enum types in `ReadReq`.
pub mod read_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(string, tag = "2")]
        pub group_name: ::prost::alloc::string::String,
        #[prost(int32, tag = "3")]
        pub buffer_size: i32,
        #[prost(message, optional, tag = "4")]
        pub uuid_option: ::core::option::Option<options::UuidOption>,
        #[prost(oneof = "options::StreamOption", tags = "1, 5")]
        pub stream_option: ::core::option::Option<options::StreamOption>,
    }
    /// Nested message and enum types in `Options`.
    pub mod options {
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct UuidOption {
            #[prost(oneof = "uuid_option::Content", tags = "1, 2")]
            pub content: ::core::option::Option<uuid_option::Content>,
        }
        /// Nested message and enum types in `UUIDOption`.
        pub mod uuid_option {
            #[derive(Clone, PartialEq, ::prost::Oneof)]
            pub enum Content {
                #[prost(message, tag = "1")]
                Structured(super::super::super::super::Empty),
                #[prost(message, tag = "2")]
                String(super::super::super::super::Empty),
            }
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum StreamOption {
            #[prost(message, tag = "1")]
            StreamIdentifier(super::super::super::StreamIdentifier),
            #[prost(message, tag = "5")]
            All(super::super::super::Empty),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Ack {
        #[prost(bytes = "vec", tag = "1")]
        pub id: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, repeated, tag = "2")]
        pub ids: ::prost::alloc::vec::Vec<super::super::Uuid>,
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Nack {
        #[prost(bytes = "vec", tag = "1")]
        pub id: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, repeated, tag = "2")]
        pub ids: ::prost::alloc::vec::Vec<super::super::Uuid>,
        #[prost(enumeration = "nack::Action", tag = "3")]
        pub action: i32,
        #[prost(string, tag = "4")]
        pub reason: ::prost::alloc::string::String,
    }
    /// Nested message and enum types in `Nack`.
    pub mod nack {
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration,
        )]
        #[repr(i32)]
        pub enum Action {
            Unknown = 0,
            Park = 1,
            Retry = 2,
            Skip = 3,
            Stop = 4,
        }
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Content {
        #[prost(message, tag = "1")]
        Options(Options),
        #[prost(message, tag = "2")]
        Ack(Ack),
        #[prost(message, tag = "3")]
        Nack(Nack),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReadResp {
    #[prost(oneof = "read_resp::Content", tags = "1, 2")]
    pub content: ::core::option::Option<read_resp::Content>,
}
/// Nested message and enum types in `ReadResp`.
pub mod read_resp {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ReadEvent {
        #[prost(message, optional, tag = "1")]
        pub event: ::core::option::Option<read_event::RecordedEvent>,
        #[prost(message, optional, tag = "2")]
        pub link: ::core::option::Option<read_event::RecordedEvent>,
        #[prost(oneof = "read_event::Position", tags = "3, 4")]
        pub position: ::core::option::Option<read_event::Position>,
        #[prost(oneof = "read_event::Count", tags = "5, 6")]
        pub count: ::core::option::Option<read_event::Count>,
    }
    /// Nested message and enum types in `ReadEvent`.
    pub mod read_event {
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct RecordedEvent {
            #[prost(message, optional, tag = "1")]
            pub id: ::core::option::Option<super::super::super::Uuid>,
            #[prost(message, optional, tag = "2")]
            pub stream_identifier: ::core::option::Option<super::super::super::StreamIdentifier>,
            #[prost(uint64, tag = "3")]
            pub stream_revision: u64,
            #[prost(uint64, tag = "4")]
            pub prepare_position: u64,
            #[prost(uint64, tag = "5")]
            pub commit_position: u64,
            #[prost(map = "string, string", tag = "6")]
            pub metadata: ::std::collections::HashMap<
                ::prost::alloc::string::String,
                ::prost::alloc::string::String,
            >,
            #[prost(bytes = "vec", tag = "7")]
            pub custom_metadata: ::prost::alloc::vec::Vec<u8>,
            #[prost(bytes = "vec", tag = "8")]
            pub data: ::prost::alloc::vec::Vec<u8>,
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Position {
            #[prost(uint64, tag = "3")]
            CommitPosition(u64),
            #[prost(message, tag = "4")]
            NoPosition(super::super::super::Empty),
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Count {
            #[prost(int32, tag = "5")]
            RetryCount(i32),
            #[prost(message, tag = "6")]
            NoRetryCount(super::super::super::Empty),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct SubscriptionConfirmation {
        #[prost(string, tag = "1")]
        pub subscription_id: ::prost::alloc::string::String,
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Content {
        #[prost(message, tag = "1")]
        Event(ReadEvent),
        #[prost(message, tag = "2")]
        SubscriptionConfirmation(SubscriptionConfirmation),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<create_req::Options>,
}
/// Nested message and enum types in `CreateReq`.
pub mod create_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[deprecated]
        #[prost(message, optional, tag = "1")]
        pub stream_identifier: ::core::option::Option<super::super::StreamIdentifier>,
        #[prost(string, tag = "2")]
        pub group_name: ::prost::alloc::string::String,
        #[prost(message, optional, tag = "3")]
        pub settings: ::core::option::Option<Settings>,
        #[prost(oneof = "options::StreamOption", tags = "4, 5")]
        pub stream_option: ::core::option::Option<options::StreamOption>,
    }
    /// Nested message and enum types in `Options`.
    pub mod options {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum StreamOption {
            #[prost(message, tag = "4")]
            Stream(super::StreamOptions),
            #[prost(message, tag = "5")]
            All(super::AllOptions),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct StreamOptions {
        #[prost(message, optional, tag = "1")]
        pub stream_identifier: ::core::option::Option<super::super::StreamIdentifier>,
        #[prost(oneof = "stream_options::RevisionOption", tags = "2, 3, 4")]
        pub revision_option: ::core::option::Option<stream_options::RevisionOption>,
    }
    /// Nested message and enum types in `StreamOptions`.
    pub mod stream_options {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum RevisionOption {
            #[prost(uint64, tag = "2")]
            Revision(u64),
            #[prost(message, tag = "3")]
            Start(super::super::super::Empty),
            #[prost(message, tag = "4")]
            End(super::super::super::Empty),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct AllOptions {
        #[prost(oneof = "all_options::AllOption", tags = "1, 2, 3")]
        pub all_option: ::core::option::Option<all_options::AllOption>,
    }
    /// Nested message and enum types in `AllOptions`.
    pub mod all_options {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum AllOption {
            #[prost(message, tag = "1")]
            Position(super::Position),
            #[prost(message, tag = "2")]
            Start(super::super::super::Empty),
            #[prost(message, tag = "3")]
            End(super::super::super::Empty),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Position {
        #[prost(uint64, tag = "1")]
        pub commit_position: u64,
        #[prost(uint64, tag = "2")]
        pub prepare_position: u64,
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Settings {
        #[prost(bool, tag = "1")]
        pub resolve_links: bool,
        #[deprecated]
        #[prost(uint64, tag = "2")]
        pub revision: u64,
        #[prost(bool, tag = "3")]
        pub extra_statistics: bool,
        #[prost(int32, tag = "5")]
        pub max_retry_count: i32,
        #[prost(int32, tag = "7")]
        pub min_checkpoint_count: i32,
        #[prost(int32, tag = "8")]
        pub max_checkpoint_count: i32,
        #[prost(int32, tag = "9")]
        pub max_subscriber_count: i32,
        #[prost(int32, tag = "10")]
        pub live_buffer_size: i32,
        #[prost(int32, tag = "11")]
        pub read_batch_size: i32,
        #[prost(int32, tag = "12")]
        pub history_buffer_size: i32,
        #[prost(enumeration = "ConsumerStrategy", tag = "13")]
        pub named_consumer_strategy: i32,
        #[prost(oneof = "settings::MessageTimeout", tags = "4, 14")]
        pub message_timeout: ::core::option::Option<settings::MessageTimeout>,
        #[prost(oneof = "settings::CheckpointAfter", tags = "6, 15")]
        pub checkpoint_after: ::core::option::Option<settings::CheckpointAfter>,
    }
    /// Nested message and enum types in `Settings`.
    pub mod settings {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum MessageTimeout {
            #[prost(int64, tag = "4")]
            MessageTimeoutTicks(i64),
            #[prost(int32, tag = "14")]
            MessageTimeoutMs(i32),
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum CheckpointAfter {
            #[prost(int64, tag = "6")]
            CheckpointAfterTicks(i64),
            #[prost(int32, tag = "15")]
            CheckpointAfterMs(i32),
        }
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum ConsumerStrategy {
        DispatchToSingle = 0,
        RoundRobin = 1,
        Pinned = 2,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateResp {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<update_req::Options>,
}
/// Nested message and enum types in `UpdateReq`.
pub mod update_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[deprecated]
        #[prost(message, optional, tag = "1")]
        pub stream_identifier: ::core::option::Option<super::super::StreamIdentifier>,
        #[prost(string, tag = "2")]
        pub group_name: ::prost::alloc::string::String,
        #[prost(message, optional, tag = "3")]
        pub settings: ::core::option::Option<Settings>,
        #[prost(oneof = "options::StreamOption", tags = "4, 5")]
        pub stream_option: ::core::option::Option<options::StreamOption>,
    }
    /// Nested message and enum types in `Options`.
    pub mod options {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum StreamOption {
            #[prost(message, tag = "4")]
            Stream(super::StreamOptions),
            #[prost(message, tag = "5")]
            All(super::AllOptions),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct StreamOptions {
        #[prost(message, optional, tag = "1")]
        pub stream_identifier: ::core::option::Option<super::super::StreamIdentifier>,
        #[prost(oneof = "stream_options::RevisionOption", tags = "2, 3, 4")]
        pub revision_option: ::core::option::Option<stream_options::RevisionOption>,
    }
    /// Nested message and enum types in `StreamOptions`.
    pub mod stream_options {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum RevisionOption {
            #[prost(uint64, tag = "2")]
            Revision(u64),
            #[prost(message, tag = "3")]
            Start(super::super::super::Empty),
            #[prost(message, tag = "4")]
            End(super::super::super::Empty),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct AllOptions {
        #[prost(oneof = "all_options::AllOption", tags = "1, 2, 3")]
        pub all_option: ::core::option::Option<all_options::AllOption>,
    }
    /// Nested message and enum types in `AllOptions`.
    pub mod all_options {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum AllOption {
            #[prost(message, tag = "1")]
            Position(super::Position),
            #[prost(message, tag = "2")]
            Start(super::super::super::Empty),
            #[prost(message, tag = "3")]
            End(super::super::super::Empty),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Position {
        #[prost(uint64, tag = "1")]
        pub commit_position: u64,
        #[prost(uint64, tag = "2")]
        pub prepare_position: u64,
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Settings {
        #[prost(bool, tag = "1")]
        pub resolve_links: bool,
        #[deprecated]
        #[prost(uint64, tag = "2")]
        pub revision: u64,
        #[prost(bool, tag = "3")]
        pub extra_statistics: bool,
        #[prost(int32, tag = "5")]
        pub max_retry_count: i32,
        #[prost(int32, tag = "7")]
        pub min_checkpoint_count: i32,
        #[prost(int32, tag = "8")]
        pub max_checkpoint_count: i32,
        #[prost(int32, tag = "9")]
        pub max_subscriber_count: i32,
        #[prost(int32, tag = "10")]
        pub live_buffer_size: i32,
        #[prost(int32, tag = "11")]
        pub read_batch_size: i32,
        #[prost(int32, tag = "12")]
        pub history_buffer_size: i32,
        #[prost(enumeration = "ConsumerStrategy", tag = "13")]
        pub named_consumer_strategy: i32,
        #[prost(oneof = "settings::MessageTimeout", tags = "4, 14")]
        pub message_timeout: ::core::option::Option<settings::MessageTimeout>,
        #[prost(oneof = "settings::CheckpointAfter", tags = "6, 15")]
        pub checkpoint_after: ::core::option::Option<settings::CheckpointAfter>,
    }
    /// Nested message and enum types in `Settings`.
    pub mod settings {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum MessageTimeout {
            #[prost(int64, tag = "4")]
            MessageTimeoutTicks(i64),
            #[prost(int32, tag = "14")]
            MessageTimeoutMs(i32),
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum CheckpointAfter {
            #[prost(int64, tag = "6")]
            CheckpointAfterTicks(i64),
            #[prost(int32, tag = "15")]
            CheckpointAfterMs(i32),
        }
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum ConsumerStrategy {
        DispatchToSingle = 0,
        RoundRobin = 1,
        Pinned = 2,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateResp {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<delete_req::Options>,
}
/// Nested message and enum types in `DeleteReq`.
pub mod delete_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(string, tag = "2")]
        pub group_name: ::prost::alloc::string::String,
        #[prost(oneof = "options::StreamOption", tags = "1, 3")]
        pub stream_option: ::core::option::Option<options::StreamOption>,
    }
    /// Nested message and enum types in `Options`.
    pub mod options {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum StreamOption {
            #[prost(message, tag = "1")]
            StreamIdentifier(super::super::super::StreamIdentifier),
            #[prost(message, tag = "3")]
            All(super::super::super::Empty),
        }
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteResp {}
#[doc = r" Generated client implementations."]
pub mod persistent_subscriptions_client {
    #![allow(unused_variables, dead_code, missing_docs)]
    use tonic::codegen::*;
    pub struct PersistentSubscriptionsClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl PersistentSubscriptionsClient<tonic::transport::Channel> {
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
    impl<T> PersistentSubscriptionsClient<T>
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
        pub async fn create(
            &mut self,
            request: impl tonic::IntoRequest<super::CreateReq>,
        ) -> Result<tonic::Response<super::CreateResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.persistent_subscriptions.PersistentSubscriptions/Create",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn update(
            &mut self,
            request: impl tonic::IntoRequest<super::UpdateReq>,
        ) -> Result<tonic::Response<super::UpdateResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.persistent_subscriptions.PersistentSubscriptions/Update",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn delete(
            &mut self,
            request: impl tonic::IntoRequest<super::DeleteReq>,
        ) -> Result<tonic::Response<super::DeleteResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.persistent_subscriptions.PersistentSubscriptions/Delete",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn read(
            &mut self,
            request: impl tonic::IntoStreamingRequest<Message = super::ReadReq>,
        ) -> Result<tonic::Response<tonic::codec::Streaming<super::ReadResp>>, tonic::Status>
        {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.persistent_subscriptions.PersistentSubscriptions/Read",
            );
            self.inner
                .streaming(request.into_streaming_request(), path, codec)
                .await
        }
    }
    impl<T: Clone> Clone for PersistentSubscriptionsClient<T> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
            }
        }
    }
    impl<T> std::fmt::Debug for PersistentSubscriptionsClient<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "PersistentSubscriptionsClient {{ ... }}")
        }
    }
}
