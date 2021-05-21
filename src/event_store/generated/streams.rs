#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReadReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<read_req::Options>,
}
/// Nested message and enum types in `ReadReq`.
pub mod read_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(enumeration = "options::ReadDirection", tag = "3")]
        pub read_direction: i32,
        #[prost(bool, tag = "4")]
        pub resolve_links: bool,
        #[prost(message, optional, tag = "9")]
        pub uuid_option: ::core::option::Option<options::UuidOption>,
        #[prost(oneof = "options::StreamOption", tags = "1, 2")]
        pub stream_option: ::core::option::Option<options::StreamOption>,
        #[prost(oneof = "options::CountOption", tags = "5, 6")]
        pub count_option: ::core::option::Option<options::CountOption>,
        #[prost(oneof = "options::FilterOption", tags = "7, 8")]
        pub filter_option: ::core::option::Option<options::FilterOption>,
    }
    /// Nested message and enum types in `Options`.
    pub mod options {
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct StreamOptions {
            #[prost(message, optional, tag = "1")]
            pub stream_identifier: ::core::option::Option<super::super::super::StreamIdentifier>,
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
                Start(super::super::super::super::Empty),
                #[prost(message, tag = "4")]
                End(super::super::super::super::Empty),
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
                Start(super::super::super::super::Empty),
                #[prost(message, tag = "3")]
                End(super::super::super::super::Empty),
            }
        }
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct SubscriptionOptions {}
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Position {
            #[prost(uint64, tag = "1")]
            pub commit_position: u64,
            #[prost(uint64, tag = "2")]
            pub prepare_position: u64,
        }
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct FilterOptions {
            #[prost(uint32, tag = "5")]
            pub checkpoint_interval_multiplier: u32,
            #[prost(oneof = "filter_options::Filter", tags = "1, 2")]
            pub filter: ::core::option::Option<filter_options::Filter>,
            #[prost(oneof = "filter_options::Window", tags = "3, 4")]
            pub window: ::core::option::Option<filter_options::Window>,
        }
        /// Nested message and enum types in `FilterOptions`.
        pub mod filter_options {
            #[derive(Clone, PartialEq, ::prost::Message)]
            pub struct Expression {
                #[prost(string, tag = "1")]
                pub regex: ::prost::alloc::string::String,
                #[prost(string, repeated, tag = "2")]
                pub prefix: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
            }
            #[derive(Clone, PartialEq, ::prost::Oneof)]
            pub enum Filter {
                #[prost(message, tag = "1")]
                StreamIdentifier(Expression),
                #[prost(message, tag = "2")]
                EventType(Expression),
            }
            #[derive(Clone, PartialEq, ::prost::Oneof)]
            pub enum Window {
                #[prost(uint32, tag = "3")]
                Max(u32),
                #[prost(message, tag = "4")]
                Count(super::super::super::super::Empty),
            }
        }
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
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration,
        )]
        #[repr(i32)]
        pub enum ReadDirection {
            Forwards = 0,
            Backwards = 1,
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum StreamOption {
            #[prost(message, tag = "1")]
            Stream(StreamOptions),
            #[prost(message, tag = "2")]
            All(AllOptions),
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum CountOption {
            #[prost(uint64, tag = "5")]
            Count(u64),
            #[prost(message, tag = "6")]
            Subscription(SubscriptionOptions),
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum FilterOption {
            #[prost(message, tag = "7")]
            Filter(FilterOptions),
            #[prost(message, tag = "8")]
            NoFilter(super::super::super::Empty),
        }
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReadResp {
    #[prost(oneof = "read_resp::Content", tags = "1, 2, 3, 4")]
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
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct SubscriptionConfirmation {
        #[prost(string, tag = "1")]
        pub subscription_id: ::prost::alloc::string::String,
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Checkpoint {
        #[prost(uint64, tag = "1")]
        pub commit_position: u64,
        #[prost(uint64, tag = "2")]
        pub prepare_position: u64,
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct StreamNotFound {
        #[prost(message, optional, tag = "1")]
        pub stream_identifier: ::core::option::Option<super::super::StreamIdentifier>,
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Content {
        #[prost(message, tag = "1")]
        Event(ReadEvent),
        #[prost(message, tag = "2")]
        Confirmation(SubscriptionConfirmation),
        #[prost(message, tag = "3")]
        Checkpoint(Checkpoint),
        #[prost(message, tag = "4")]
        StreamNotFound(StreamNotFound),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AppendReq {
    #[prost(oneof = "append_req::Content", tags = "1, 2")]
    pub content: ::core::option::Option<append_req::Content>,
}
/// Nested message and enum types in `AppendReq`.
pub mod append_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(message, optional, tag = "1")]
        pub stream_identifier: ::core::option::Option<super::super::StreamIdentifier>,
        #[prost(oneof = "options::ExpectedStreamRevision", tags = "2, 3, 4, 5")]
        pub expected_stream_revision: ::core::option::Option<options::ExpectedStreamRevision>,
    }
    /// Nested message and enum types in `Options`.
    pub mod options {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum ExpectedStreamRevision {
            #[prost(uint64, tag = "2")]
            Revision(u64),
            #[prost(message, tag = "3")]
            NoStream(super::super::super::Empty),
            #[prost(message, tag = "4")]
            Any(super::super::super::Empty),
            #[prost(message, tag = "5")]
            StreamExists(super::super::super::Empty),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ProposedMessage {
        #[prost(message, optional, tag = "1")]
        pub id: ::core::option::Option<super::super::Uuid>,
        #[prost(map = "string, string", tag = "2")]
        pub metadata: ::std::collections::HashMap<
            ::prost::alloc::string::String,
            ::prost::alloc::string::String,
        >,
        #[prost(bytes = "vec", tag = "3")]
        pub custom_metadata: ::prost::alloc::vec::Vec<u8>,
        #[prost(bytes = "vec", tag = "4")]
        pub data: ::prost::alloc::vec::Vec<u8>,
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Content {
        #[prost(message, tag = "1")]
        Options(Options),
        #[prost(message, tag = "2")]
        ProposedMessage(ProposedMessage),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AppendResp {
    #[prost(oneof = "append_resp::Result", tags = "1, 2")]
    pub result: ::core::option::Option<append_resp::Result>,
}
/// Nested message and enum types in `AppendResp`.
pub mod append_resp {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Position {
        #[prost(uint64, tag = "1")]
        pub commit_position: u64,
        #[prost(uint64, tag = "2")]
        pub prepare_position: u64,
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Success {
        #[prost(oneof = "success::CurrentRevisionOption", tags = "1, 2")]
        pub current_revision_option: ::core::option::Option<success::CurrentRevisionOption>,
        #[prost(oneof = "success::PositionOption", tags = "3, 4")]
        pub position_option: ::core::option::Option<success::PositionOption>,
    }
    /// Nested message and enum types in `Success`.
    pub mod success {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum CurrentRevisionOption {
            #[prost(uint64, tag = "1")]
            CurrentRevision(u64),
            #[prost(message, tag = "2")]
            NoStream(super::super::super::Empty),
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum PositionOption {
            #[prost(message, tag = "3")]
            Position(super::Position),
            #[prost(message, tag = "4")]
            NoPosition(super::super::super::Empty),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct WrongExpectedVersion {
        #[prost(
            oneof = "wrong_expected_version::CurrentRevisionOption2060",
            tags = "1, 2"
        )]
        pub current_revision_option_20_6_0:
            ::core::option::Option<wrong_expected_version::CurrentRevisionOption2060>,
        #[prost(
            oneof = "wrong_expected_version::ExpectedRevisionOption2060",
            tags = "3, 4, 5"
        )]
        pub expected_revision_option_20_6_0:
            ::core::option::Option<wrong_expected_version::ExpectedRevisionOption2060>,
        #[prost(oneof = "wrong_expected_version::CurrentRevisionOption", tags = "6, 7")]
        pub current_revision_option:
            ::core::option::Option<wrong_expected_version::CurrentRevisionOption>,
        #[prost(
            oneof = "wrong_expected_version::ExpectedRevisionOption",
            tags = "8, 9, 10, 11"
        )]
        pub expected_revision_option:
            ::core::option::Option<wrong_expected_version::ExpectedRevisionOption>,
    }
    /// Nested message and enum types in `WrongExpectedVersion`.
    pub mod wrong_expected_version {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum CurrentRevisionOption2060 {
            #[prost(uint64, tag = "1")]
            CurrentRevision2060(u64),
            #[prost(message, tag = "2")]
            NoStream2060(super::super::super::Empty),
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum ExpectedRevisionOption2060 {
            #[prost(uint64, tag = "3")]
            ExpectedRevision2060(u64),
            #[prost(message, tag = "4")]
            Any2060(super::super::super::Empty),
            #[prost(message, tag = "5")]
            StreamExists2060(super::super::super::Empty),
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum CurrentRevisionOption {
            #[prost(uint64, tag = "6")]
            CurrentRevision(u64),
            #[prost(message, tag = "7")]
            CurrentNoStream(super::super::super::Empty),
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum ExpectedRevisionOption {
            #[prost(uint64, tag = "8")]
            ExpectedRevision(u64),
            #[prost(message, tag = "9")]
            ExpectedAny(super::super::super::Empty),
            #[prost(message, tag = "10")]
            ExpectedStreamExists(super::super::super::Empty),
            #[prost(message, tag = "11")]
            ExpectedNoStream(super::super::super::Empty),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        Success(Success),
        #[prost(message, tag = "2")]
        WrongExpectedVersion(WrongExpectedVersion),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BatchAppendReq {
    #[prost(message, optional, tag = "1")]
    pub correlation_id: ::core::option::Option<super::Uuid>,
    #[prost(message, optional, tag = "2")]
    pub options: ::core::option::Option<batch_append_req::Options>,
    #[prost(message, repeated, tag = "3")]
    pub proposed_messages: ::prost::alloc::vec::Vec<batch_append_req::ProposedMessage>,
    #[prost(bool, tag = "4")]
    pub is_final: bool,
}
/// Nested message and enum types in `BatchAppendReq`.
pub mod batch_append_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(message, optional, tag = "1")]
        pub stream_identifier: ::core::option::Option<super::super::StreamIdentifier>,
        #[prost(message, optional, tag = "6")]
        pub deadline: ::core::option::Option<::prost_types::Timestamp>,
        #[prost(oneof = "options::ExpectedStreamPosition", tags = "2, 3, 4, 5")]
        pub expected_stream_position: ::core::option::Option<options::ExpectedStreamPosition>,
    }
    /// Nested message and enum types in `Options`.
    pub mod options {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum ExpectedStreamPosition {
            #[prost(uint64, tag = "2")]
            StreamPosition(u64),
            #[prost(message, tag = "3")]
            NoStream(()),
            #[prost(message, tag = "4")]
            Any(()),
            #[prost(message, tag = "5")]
            StreamExists(()),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ProposedMessage {
        #[prost(message, optional, tag = "1")]
        pub id: ::core::option::Option<super::super::Uuid>,
        #[prost(map = "string, string", tag = "2")]
        pub metadata: ::std::collections::HashMap<
            ::prost::alloc::string::String,
            ::prost::alloc::string::String,
        >,
        #[prost(bytes = "vec", tag = "3")]
        pub custom_metadata: ::prost::alloc::vec::Vec<u8>,
        #[prost(bytes = "vec", tag = "4")]
        pub data: ::prost::alloc::vec::Vec<u8>,
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BatchAppendResp {
    #[prost(message, optional, tag = "1")]
    pub correlation_id: ::core::option::Option<super::Uuid>,
    #[prost(message, optional, tag = "4")]
    pub stream_identifier: ::core::option::Option<super::StreamIdentifier>,
    #[prost(oneof = "batch_append_resp::Result", tags = "2, 3")]
    pub result: ::core::option::Option<batch_append_resp::Result>,
    #[prost(
        oneof = "batch_append_resp::ExpectedStreamPosition",
        tags = "5, 6, 7, 8"
    )]
    pub expected_stream_position: ::core::option::Option<batch_append_resp::ExpectedStreamPosition>,
}
/// Nested message and enum types in `BatchAppendResp`.
pub mod batch_append_resp {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Success {
        #[prost(oneof = "success::CurrentRevisionOption", tags = "1, 2")]
        pub current_revision_option: ::core::option::Option<success::CurrentRevisionOption>,
        #[prost(oneof = "success::PositionOption", tags = "3, 4")]
        pub position_option: ::core::option::Option<success::PositionOption>,
    }
    /// Nested message and enum types in `Success`.
    pub mod success {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum CurrentRevisionOption {
            #[prost(uint64, tag = "1")]
            CurrentRevision(u64),
            #[prost(message, tag = "2")]
            NoStream(()),
        }
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum PositionOption {
            #[prost(message, tag = "3")]
            Position(super::super::super::AllStreamPosition),
            #[prost(message, tag = "4")]
            NoPosition(()),
        }
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "2")]
        Error(super::super::super::super::google::rpc::Status),
        #[prost(message, tag = "3")]
        Success(Success),
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum ExpectedStreamPosition {
        #[prost(uint64, tag = "5")]
        StreamPosition(u64),
        #[prost(message, tag = "6")]
        NoStream(()),
        #[prost(message, tag = "7")]
        Any(()),
        #[prost(message, tag = "8")]
        StreamExists(()),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<delete_req::Options>,
}
/// Nested message and enum types in `DeleteReq`.
pub mod delete_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(message, optional, tag = "1")]
        pub stream_identifier: ::core::option::Option<super::super::StreamIdentifier>,
        #[prost(oneof = "options::ExpectedStreamRevision", tags = "2, 3, 4, 5")]
        pub expected_stream_revision: ::core::option::Option<options::ExpectedStreamRevision>,
    }
    /// Nested message and enum types in `Options`.
    pub mod options {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum ExpectedStreamRevision {
            #[prost(uint64, tag = "2")]
            Revision(u64),
            #[prost(message, tag = "3")]
            NoStream(super::super::super::Empty),
            #[prost(message, tag = "4")]
            Any(super::super::super::Empty),
            #[prost(message, tag = "5")]
            StreamExists(super::super::super::Empty),
        }
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteResp {
    #[prost(oneof = "delete_resp::PositionOption", tags = "1, 2")]
    pub position_option: ::core::option::Option<delete_resp::PositionOption>,
}
/// Nested message and enum types in `DeleteResp`.
pub mod delete_resp {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Position {
        #[prost(uint64, tag = "1")]
        pub commit_position: u64,
        #[prost(uint64, tag = "2")]
        pub prepare_position: u64,
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum PositionOption {
        #[prost(message, tag = "1")]
        Position(Position),
        #[prost(message, tag = "2")]
        NoPosition(super::super::Empty),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TombstoneReq {
    #[prost(message, optional, tag = "1")]
    pub options: ::core::option::Option<tombstone_req::Options>,
}
/// Nested message and enum types in `TombstoneReq`.
pub mod tombstone_req {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Options {
        #[prost(message, optional, tag = "1")]
        pub stream_identifier: ::core::option::Option<super::super::StreamIdentifier>,
        #[prost(oneof = "options::ExpectedStreamRevision", tags = "2, 3, 4, 5")]
        pub expected_stream_revision: ::core::option::Option<options::ExpectedStreamRevision>,
    }
    /// Nested message and enum types in `Options`.
    pub mod options {
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum ExpectedStreamRevision {
            #[prost(uint64, tag = "2")]
            Revision(u64),
            #[prost(message, tag = "3")]
            NoStream(super::super::super::Empty),
            #[prost(message, tag = "4")]
            Any(super::super::super::Empty),
            #[prost(message, tag = "5")]
            StreamExists(super::super::super::Empty),
        }
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TombstoneResp {
    #[prost(oneof = "tombstone_resp::PositionOption", tags = "1, 2")]
    pub position_option: ::core::option::Option<tombstone_resp::PositionOption>,
}
/// Nested message and enum types in `TombstoneResp`.
pub mod tombstone_resp {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Position {
        #[prost(uint64, tag = "1")]
        pub commit_position: u64,
        #[prost(uint64, tag = "2")]
        pub prepare_position: u64,
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum PositionOption {
        #[prost(message, tag = "1")]
        Position(Position),
        #[prost(message, tag = "2")]
        NoPosition(super::super::Empty),
    }
}
#[doc = r" Generated client implementations."]
pub mod streams_client {
    #![allow(unused_variables, dead_code, missing_docs)]
    use tonic::codegen::*;
    pub struct StreamsClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl StreamsClient<tonic::transport::Channel> {
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
    impl<T> StreamsClient<T>
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
            request: impl tonic::IntoRequest<super::ReadReq>,
        ) -> Result<tonic::Response<tonic::codec::Streaming<super::ReadResp>>, tonic::Status>
        {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path =
                http::uri::PathAndQuery::from_static("/event_store.client.streams.Streams/Read");
            self.inner
                .server_streaming(request.into_request(), path, codec)
                .await
        }
        pub async fn append(
            &mut self,
            request: impl tonic::IntoStreamingRequest<Message = super::AppendReq>,
        ) -> Result<tonic::Response<super::AppendResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path =
                http::uri::PathAndQuery::from_static("/event_store.client.streams.Streams/Append");
            self.inner
                .client_streaming(request.into_streaming_request(), path, codec)
                .await
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
            let path =
                http::uri::PathAndQuery::from_static("/event_store.client.streams.Streams/Delete");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn tombstone(
            &mut self,
            request: impl tonic::IntoRequest<super::TombstoneReq>,
        ) -> Result<tonic::Response<super::TombstoneResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.streams.Streams/Tombstone",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn batch_append(
            &mut self,
            request: impl tonic::IntoStreamingRequest<Message = super::BatchAppendReq>,
        ) -> Result<tonic::Response<tonic::codec::Streaming<super::BatchAppendResp>>, tonic::Status>
        {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/event_store.client.streams.Streams/BatchAppend",
            );
            self.inner
                .streaming(request.into_streaming_request(), path, codec)
                .await
        }
    }
    impl<T: Clone> Clone for StreamsClient<T> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
            }
        }
    }
    impl<T> std::fmt::Debug for StreamsClient<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "StreamsClient {{ ... }}")
        }
    }
}
