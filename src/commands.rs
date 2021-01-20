//! Commands this client supports.
use futures::{stream, TryStreamExt};
use futures::{Stream, StreamExt};

use crate::event_store::client::{persistent, shared, streams};
use crate::types::{
    EventData, ExpectedRevision, ExpectedVersion, PersistentSubscriptionSettings, Position,
    ReadDirection, RecordedEvent, ResolvedEvent, Revision, SubEvent, WriteResult,
    WrongExpectedVersion,
};

use persistent::persistent_subscriptions_client::PersistentSubscriptionsClient;
use shared::{Empty, StreamIdentifier, Uuid};
use std::marker::Unpin;
use streams::append_req::options::ExpectedStreamRevision;
use streams::streams_client::StreamsClient;

use crate::grpc::GrpcClient;
use crate::{Credentials, CurrentRevision, LinkTos, NakAction, ReadResult, SystemConsumerStrategy};
use tonic::Request;

fn convert_expected_version(version: ExpectedVersion) -> ExpectedStreamRevision {
    match version {
        ExpectedVersion::Any => ExpectedStreamRevision::Any(Empty {}),
        ExpectedVersion::StreamExists => ExpectedStreamRevision::StreamExists(Empty {}),
        ExpectedVersion::NoStream => ExpectedStreamRevision::NoStream(Empty {}),
        ExpectedVersion::Exact(version) => ExpectedStreamRevision::Revision(version),
    }
}

fn raw_uuid_to_uuid(src: Uuid) -> uuid::Uuid {
    use byteorder::{BigEndian, ByteOrder};

    let value = src
        .value
        .expect("We expect Uuid value to be defined for now");

    match value {
        shared::uuid::Value::Structured(s) => {
            let mut buf = vec![];

            BigEndian::write_i64(&mut buf, s.most_significant_bits);
            BigEndian::write_i64(&mut buf, s.least_significant_bits);

            uuid::Uuid::from_slice(buf.as_slice())
                .expect("We expect a valid UUID out of byte buffer")
        }

        shared::uuid::Value::String(s) => s
            .parse()
            .expect("We expect a valid UUID out of this String"),
    }
}

fn raw_persistent_uuid_to_uuid(src: Uuid) -> uuid::Uuid {
    use byteorder::{BigEndian, ByteOrder};

    let value = src
        .value
        .expect("We expect Uuid value to be defined for now");

    match value {
        shared::uuid::Value::Structured(s) => {
            let mut buf = vec![];

            BigEndian::write_i64(&mut buf, s.most_significant_bits);
            BigEndian::write_i64(&mut buf, s.least_significant_bits);

            uuid::Uuid::from_slice(buf.as_slice())
                .expect("We expect a valid UUID out of byte buffer")
        }

        shared::uuid::Value::String(s) => s
            .parse()
            .expect("We expect a valid UUID out of this String"),
    }
}

fn convert_event_data(event: EventData) -> streams::AppendReq {
    use streams::append_req;

    let id = event.id_opt.unwrap_or_else(uuid::Uuid::new_v4);
    let id = shared::uuid::Value::String(id.to_string());
    let id = Uuid { value: Some(id) };
    let custom_metadata = event
        .custom_metadata
        .map_or_else(Vec::new, |b| (&*b).into());

    let msg = append_req::ProposedMessage {
        id: Some(id),
        metadata: event.metadata,
        custom_metadata,
        data: (&*event.payload).into(),
    };

    let content = append_req::Content::ProposedMessage(msg);

    streams::AppendReq {
        content: Some(content),
    }
}

fn convert_proto_recorded_event(
    event: streams::read_resp::read_event::RecordedEvent,
) -> RecordedEvent {
    let id = event
        .id
        .map(raw_uuid_to_uuid)
        .expect("Unable to parse Uuid [convert_proto_recorded_event]");

    let position = Position {
        commit: event.commit_position,
        prepare: event.prepare_position,
    };

    let event_type = if let Some(tpe) = event.metadata.get("type") {
        tpe.to_string()
    } else {
        "<no-event-type-provided>".to_string()
    };

    let is_json = if let Some(content_type) = event.metadata.get("content-type") {
        matches!(content_type.as_str(), "application/json")
    } else {
        false
    };

    let stream_id = String::from_utf8(
        event
            .stream_identifier
            .expect("stream_identifier is always defined")
            .stream_name,
    )
    .expect("It's always UTF-8");
    RecordedEvent {
        id,
        stream_id,
        revision: event.stream_revision,
        position,
        event_type,
        is_json,
        metadata: event.metadata,
        custom_metadata: event.custom_metadata.into(),
        data: event.data.into(),
    }
}

fn convert_persistent_proto_recorded_event(
    event: persistent::read_resp::read_event::RecordedEvent,
) -> RecordedEvent {
    let id = event
        .id
        .map(raw_persistent_uuid_to_uuid)
        .expect("Unable to parse Uuid [convert_persistent_proto_recorded_event]");

    let position = Position {
        commit: event.commit_position,
        prepare: event.prepare_position,
    };

    let event_type = if let Some(tpe) = event.metadata.get("type") {
        tpe.to_string()
    } else {
        "<no-event-type-provided>".to_owned()
    };

    let is_json = if let Some(content_type) = event.metadata.get("content-type") {
        matches!(content_type.as_str(), "application/json")
    } else {
        false
    };

    let stream_id = String::from_utf8(
        event
            .stream_identifier
            .expect("stream_identifier is always defined")
            .stream_name,
    )
    .expect("string is UTF-8 valid");

    RecordedEvent {
        id,
        stream_id,
        revision: event.stream_revision,
        position,
        event_type,
        is_json,
        metadata: event.metadata,
        custom_metadata: event.custom_metadata.into(),
        data: event.data.into(),
    }
}

fn convert_settings_create(
    settings: PersistentSubscriptionSettings,
) -> persistent::create_req::Settings {
    let named_consumer_strategy = match settings.named_consumer_strategy {
        SystemConsumerStrategy::DispatchToSingle => 0,
        SystemConsumerStrategy::RoundRobin => 1,
        SystemConsumerStrategy::Pinned => 2,
    };

    persistent::create_req::Settings {
        resolve_links: settings.resolve_links,
        revision: settings.revision,
        extra_statistics: settings.extra_stats,
        message_timeout: Some(
            persistent::create_req::settings::MessageTimeout::MessageTimeoutMs(
                settings.message_timeout.as_millis() as i32,
            ),
        ),
        max_retry_count: settings.max_retry_count,
        checkpoint_after: Some(
            persistent::create_req::settings::CheckpointAfter::CheckpointAfterMs(
                settings.checkpoint_after.as_millis() as i32,
            ),
        ),
        min_checkpoint_count: settings.min_checkpoint_count,
        max_checkpoint_count: settings.max_checkpoint_count,
        max_subscriber_count: settings.max_subscriber_count,
        live_buffer_size: settings.live_buffer_size,
        read_batch_size: settings.read_batch_size,
        history_buffer_size: settings.history_buffer_size,
        named_consumer_strategy,
    }
}

fn convert_settings_update(
    settings: PersistentSubscriptionSettings,
) -> persistent::update_req::Settings {
    let named_consumer_strategy = match settings.named_consumer_strategy {
        SystemConsumerStrategy::DispatchToSingle => 0,
        SystemConsumerStrategy::RoundRobin => 1,
        SystemConsumerStrategy::Pinned => 2,
    };

    persistent::update_req::Settings {
        resolve_links: settings.resolve_links,
        revision: settings.revision,
        extra_statistics: settings.extra_stats,
        message_timeout: Some(
            persistent::update_req::settings::MessageTimeout::MessageTimeoutMs(
                settings.message_timeout.as_millis() as i32,
            ),
        ),
        max_retry_count: settings.max_retry_count,
        checkpoint_after: Some(
            persistent::update_req::settings::CheckpointAfter::CheckpointAfterMs(
                settings.checkpoint_after.as_millis() as i32,
            ),
        ),
        min_checkpoint_count: settings.min_checkpoint_count,
        max_checkpoint_count: settings.max_checkpoint_count,
        max_subscriber_count: settings.max_subscriber_count,
        live_buffer_size: settings.live_buffer_size,
        read_batch_size: settings.read_batch_size,
        history_buffer_size: settings.history_buffer_size,
        named_consumer_strategy,
    }
}

fn convert_proto_read_event(event: streams::read_resp::ReadEvent) -> ResolvedEvent {
    let commit_position = if let Some(pos_alt) = event.position {
        match pos_alt {
            streams::read_resp::read_event::Position::CommitPosition(pos) => Some(pos),
            streams::read_resp::read_event::Position::NoPosition(_) => None,
        }
    } else {
        None
    };

    ResolvedEvent {
        event: event.event.map(convert_proto_recorded_event),
        link: event.link.map(convert_proto_recorded_event),
        commit_position,
    }
}

fn convert_persistent_proto_read_event(event: persistent::read_resp::ReadEvent) -> ResolvedEvent {
    let commit_position = if let Some(pos_alt) = event.position {
        match pos_alt {
            persistent::read_resp::read_event::Position::CommitPosition(pos) => Some(pos),
            persistent::read_resp::read_event::Position::NoPosition(_) => None,
        }
    } else {
        None
    };

    ResolvedEvent {
        event: event.event.map(convert_persistent_proto_recorded_event),
        link: event.link.map(convert_persistent_proto_recorded_event),
        commit_position,
    }
}

fn configure_auth_req<A>(req: &mut Request<A>, creds_opt: Option<Credentials>) {
    use tonic::metadata::MetadataValue;

    if let Some(creds) = creds_opt {
        let login = String::from_utf8_lossy(&*creds.login).into_owned();
        let password = String::from_utf8_lossy(&*creds.password).into_owned();

        let basic_auth_string = base64::encode(&format!("{}:{}", login, password));
        let basic_auth = format!("Basic {}", basic_auth_string);
        let header_value = MetadataValue::from_str(basic_auth.as_str())
            .expect("Auth header value should be valid metadata header value");

        req.metadata_mut().insert("authorization", header_value);
    }
}

#[derive(Debug, Clone)]
pub struct FilterConf {
    based_on_stream: bool,
    max: Option<u32>,
    regex: Option<String>,
    prefixes: Vec<String>,
}

impl FilterConf {
    pub fn based_on_stream_name() -> Self {
        FilterConf {
            based_on_stream: true,
            max: None,
            regex: None,
            prefixes: Vec::new(),
        }
    }

    pub fn based_on_event_type() -> Self {
        let mut temp = FilterConf::based_on_stream_name();
        temp.based_on_stream = false;

        temp
    }

    pub fn max(self, max: u32) -> Self {
        FilterConf {
            max: Some(max),
            ..self
        }
    }

    pub fn regex<A: AsRef<str>>(self, regex: A) -> Self {
        FilterConf {
            regex: Some(regex.as_ref().to_string()),
            ..self
        }
    }

    pub fn add_prefix<A: AsRef<str>>(mut self, prefix: A) -> Self {
        self.prefixes.push(prefix.as_ref().to_string());
        self
    }

    pub fn into_proto(self) -> streams::read_req::options::FilterOptions {
        use options::filter_options::{Expression, Filter, Window};
        use streams::read_req::options::{self, FilterOptions};

        let window = match self.max {
            Some(max) => Window::Max(max),
            None => Window::Count(Empty {}),
        };

        let expr = Expression {
            regex: self.regex.unwrap_or_else(|| "".to_string()),
            prefix: self.prefixes,
        };

        let filter = if self.based_on_stream {
            Filter::StreamIdentifier(expr)
        } else {
            Filter::EventType(expr)
        };

        FilterOptions {
            filter: Some(filter),
            window: Some(window),
            checkpoint_interval_multiplier: 1,
        }
    }
}

/// Command that sends events to a given stream.
pub struct WriteEvents {
    connection: GrpcClient,
    stream: String,
    version: ExpectedVersion,
    creds: Option<Credentials>,
}

impl WriteEvents {
    pub(crate) fn new(connection: GrpcClient, stream: String, creds: Option<Credentials>) -> Self {
        WriteEvents {
            connection,
            stream,
            version: ExpectedVersion::Any,
            creds,
        }
    }

    /// Asks the server to check that the stream receiving the event is at
    /// the given expected version. Default: `Credentials::Any`.
    pub fn expected_version(self, version: ExpectedVersion) -> Self {
        WriteEvents { version, ..self }
    }

    /// Performs the command with the given credentials.
    pub fn credentials(self, creds: Credentials) -> Self {
        WriteEvents {
            creds: Some(creds),
            ..self
        }
    }

    /// Sends asynchronously the write command to the server.
    pub async fn send_event(
        self,
        event: EventData,
    ) -> crate::Result<Result<WriteResult, WrongExpectedVersion>> {
        self.send_iter(vec![event]).await
    }

    /// Sends asynchronously the write command to the server.
    pub async fn send_iter<I>(
        self,
        events: I,
    ) -> crate::Result<Result<WriteResult, WrongExpectedVersion>>
    where
        I: IntoIterator<Item = EventData> + Send + Sync,
        <I as IntoIterator>::IntoIter: Send + Sync + 'static,
    {
        self.send(futures::stream::iter(events)).await
    }

    /// Sends asynchronously the write command to the server.
    pub async fn send<S>(
        self,
        events: S,
    ) -> crate::Result<Result<WriteResult, WrongExpectedVersion>>
    where
        S: Stream<Item = EventData> + Send + Sync + 'static,
    {
        use streams::append_req::{self, Content};
        use streams::AppendReq;

        let stream = self.stream;
        let version = self.version;
        let creds = self.creds;

        self.connection.execute(move |channel| async move {
            let stream_identifier = Some(StreamIdentifier {
                stream_name: stream.into_bytes(),
            });
            let header = Content::Options(append_req::Options {
                stream_identifier,
                expected_stream_revision: Some(convert_expected_version(version)),
            });
            let header = AppendReq {
                content: Some(header),
            };
            let header = stream::once(async move { header });
            let events = events.map(convert_event_data);
            let payload = header.chain(events);
            let mut req = Request::new(payload);

            configure_auth_req(&mut req, creds);

            let mut client = StreamsClient::new(channel);
            let resp = client.append(req).await?.into_inner();

            match resp.result.unwrap() {
                streams::append_resp::Result::Success(success) => {
                    let next_expected_version = match success.current_revision_option.unwrap() {
                        streams::append_resp::success::CurrentRevisionOption::CurrentRevision(rev) => {
                            rev
                        }
                        streams::append_resp::success::CurrentRevisionOption::NoStream(_) => 0,
                    };

                    let position = match success.position_option.unwrap() {
                        streams::append_resp::success::PositionOption::Position(pos) => Position {
                            commit: pos.commit_position,
                            prepare: pos.prepare_position,
                        },

                        streams::append_resp::success::PositionOption::NoPosition(_) => {
                            Position::start()
                        }
                    };

                    let write_result = WriteResult {
                        next_expected_version,
                        position,
                    };

                    Ok(Ok(write_result))
                }

                streams::append_resp::Result::WrongExpectedVersion(error) => {
                    let current = match error.current_revision_option.unwrap() {
                        streams::append_resp::wrong_expected_version::CurrentRevisionOption::CurrentRevision(rev) => CurrentRevision::Current(rev),
                        streams::append_resp::wrong_expected_version::CurrentRevisionOption::NoStream(_) => CurrentRevision::NoStream,
                    };

                    let expected = match error.expected_revision_option.unwrap() {
                        streams::append_resp::wrong_expected_version::ExpectedRevisionOption::ExpectedRevision(rev) => ExpectedRevision::Expected(rev),
                        streams::append_resp::wrong_expected_version::ExpectedRevisionOption::Any(_) => ExpectedRevision::Any,
                        streams::append_resp::wrong_expected_version::ExpectedRevisionOption::StreamExists(_) => ExpectedRevision::StreamExists,
                    };

                    Ok(Err(WrongExpectedVersion { current, expected }))
                }
            }
        }).await
    }
}

/// A command that reads several events from a stream. It can read events
/// forward or backward.
pub struct ReadStreamEvents {
    connection: GrpcClient,
    stream: String,
    revision: Revision<u64>,
    resolve_link_tos: bool,
    direction: ReadDirection,
    creds: Option<Credentials>,
}

impl ReadStreamEvents {
    pub(crate) fn new(connection: GrpcClient, stream: String, creds: Option<Credentials>) -> Self {
        ReadStreamEvents {
            connection,
            stream,
            revision: Revision::Start,
            resolve_link_tos: false,
            direction: ReadDirection::Forward,
            creds,
        }
    }

    /// Asks the command to read forward (toward the end of the stream).
    /// That's the default behavior.
    pub fn forward(self) -> Self {
        self.set_direction(ReadDirection::Forward)
    }

    /// Asks the command to read backward (toward the begining of the stream).
    pub fn backward(self) -> Self {
        self.set_direction(ReadDirection::Backward)
    }

    fn set_direction(self, direction: ReadDirection) -> Self {
        ReadStreamEvents { direction, ..self }
    }

    /// Performs the command with the given credentials.
    pub fn credentials(self, value: Credentials) -> Self {
        ReadStreamEvents {
            creds: Some(value),
            ..self
        }
    }

    /// Performs the command with the given credentials.
    pub fn set_credentials(self, creds: Option<Credentials>) -> Self {
        ReadStreamEvents { creds, ..self }
    }

    /// Starts the read at the given event number. By default, it starts at
    /// 0.
    pub fn start_from(self, start: u64) -> Self {
        ReadStreamEvents {
            revision: Revision::Exact(start),
            ..self
        }
    }

    /// Starts the read from the beginning of the stream. It also set the read
    /// direction to `Forward`.
    pub fn start_from_beginning(self) -> Self {
        ReadStreamEvents {
            revision: Revision::Start,
            direction: ReadDirection::Forward,
            ..self
        }
    }

    /// Starts the read from the end of the stream. It also set the read
    /// direction to `Backward`.
    pub fn start_from_end_of_stream(self) -> Self {
        ReadStreamEvents {
            revision: Revision::End,
            direction: ReadDirection::Backward,
            ..self
        }
    }

    /// When using projections, you can have links placed into another stream.
    /// If you set `true`, the server will resolve those links and will return
    /// the event that the link points to. Default: [NoResolution](../types/enum.LinkTos.html).
    pub fn resolve_link_tos(self, tos: LinkTos) -> Self {
        let resolve_link_tos = tos.raw_resolve_lnk_tos();

        ReadStreamEvents {
            resolve_link_tos,
            ..self
        }
    }

    /// Sends asynchronously the read command to the server.
    pub async fn execute(
        self,
        count: u64,
    ) -> crate::Result<
        ReadResult<Box<dyn Stream<Item = crate::Result<ResolvedEvent>> + Send + Unpin>>,
    > {
        use streams::read_req::options::stream_options::RevisionOption;
        use streams::read_req::options::{self, StreamOption, StreamOptions};
        use streams::read_req::Options;

        let read_direction = match self.direction {
            ReadDirection::Forward => 0,
            ReadDirection::Backward => 1,
        };

        let revision_option = match self.revision {
            Revision::Exact(rev) => RevisionOption::Revision(rev),
            Revision::Start => RevisionOption::Start(Empty {}),
            Revision::End => RevisionOption::End(Empty {}),
        };

        let stream_identifier = Some(StreamIdentifier {
            stream_name: self.stream.into_bytes(),
        });
        let stream_options = StreamOptions {
            stream_identifier,
            revision_option: Some(revision_option),
        };

        let uuid_option = options::UuidOption {
            content: Some(options::uuid_option::Content::String(Empty {})),
        };

        let options = Options {
            stream_option: Some(StreamOption::Stream(stream_options)),
            resolve_links: self.resolve_link_tos,
            filter_option: Some(options::FilterOption::NoFilter(Empty {})),
            count_option: Some(options::CountOption::Count(count)),
            uuid_option: Some(uuid_option),
            read_direction,
        };

        let req = streams::ReadReq {
            options: Some(options),
        };

        let mut req = Request::new(req);

        configure_auth_req(&mut req, self.creds);

        self.connection
            .execute(|channel| async {
                let mut client = StreamsClient::new(channel);
                let mut stream = client.read(req).await?.into_inner();

                if let Some(resp) = stream.try_next().await? {
                    match resp.content.as_ref().unwrap() {
                        streams::read_resp::Content::StreamNotFound(params) => {
                            let stream_name = std::string::String::from_utf8(
                                params
                                    .stream_identifier
                                    .as_ref()
                                    .unwrap()
                                    .stream_name
                                    .clone(),
                            )
                            .expect("Don't worry this string is valid!");

                            return Ok(ReadResult::StreamNotFound(stream_name));
                        }

                        _ => {
                            let stream = stream::once(futures::future::ok::<
                                streams::ReadResp,
                                tonic::Status,
                            >(resp))
                            .chain(stream)
                            .try_filter_map(|resp| {
                                let value = match resp.content.unwrap() {
                                    streams::read_resp::Content::Event(event) => {
                                        Some(convert_proto_read_event(event))
                                    }
                                    _ => None,
                                };

                                futures::future::ok(value)
                            })
                            .map_err(crate::Error::from_grpc);

                            let stream: Box<
                                dyn Stream<Item = crate::Result<ResolvedEvent>> + Send + Unpin,
                            > = Box::new(stream);

                            return Ok(ReadResult::Ok(stream));
                        }
                    }
                }

                Ok(ReadResult::Ok(Box::new(stream::empty())))
            })
            .await
    }

    /// Reads all the events of a stream.
    pub async fn read_through(
        self,
    ) -> crate::Result<
        ReadResult<Box<dyn Stream<Item = crate::Result<ResolvedEvent>> + Send + Unpin>>,
    > {
        self.execute(u64::MAX).await
    }
}

/// Like `ReadStreamEvents` but specialized to system stream '$all'.
pub struct ReadAllEvents {
    connection: GrpcClient,
    revision: Revision<Position>,
    resolve_link_tos: bool,
    direction: ReadDirection,
    creds: Option<Credentials>,
}

impl ReadAllEvents {
    pub(crate) fn new(connection: GrpcClient, creds: Option<Credentials>) -> Self {
        ReadAllEvents {
            connection,
            revision: Revision::Start,
            resolve_link_tos: false,
            direction: ReadDirection::Forward,
            creds,
        }
    }

    /// Asks the command to read forward (toward the end of the stream).
    /// That's the default behavior.
    pub fn forward(self) -> Self {
        self.set_direction(ReadDirection::Forward)
    }

    /// Asks the command to read backward (toward the begining of the stream).
    pub fn backward(self) -> Self {
        self.set_direction(ReadDirection::Backward)
    }

    fn set_direction(self, direction: ReadDirection) -> Self {
        ReadAllEvents { direction, ..self }
    }

    /// Performs the command with the given credentials.
    pub fn credentials(self, value: Credentials) -> Self {
        ReadAllEvents {
            creds: Some(value),
            ..self
        }
    }

    /// Starts the read ot the given event number. By default, it starts at
    /// `types::Position::start`.
    pub fn start_from(self, start: Position) -> Self {
        let revision = Revision::Exact(start);
        ReadAllEvents { revision, ..self }
    }

    /// Starts the read from the beginning of the stream. It also set the read
    /// direction to `Forward`.
    pub fn start_from_beginning(self) -> Self {
        let revision = Revision::Start;
        let direction = ReadDirection::Forward;

        ReadAllEvents {
            revision,
            direction,
            ..self
        }
    }

    /// Starts the read from the end of the stream. It also set the read
    /// direction to `Backward`.
    pub fn start_from_end_of_stream(self) -> Self {
        let revision = Revision::End;
        let direction = ReadDirection::Backward;

        ReadAllEvents {
            revision,
            direction,
            ..self
        }
    }

    /// When using projections, you can have links placed into another stream.
    /// If you set `true`, the server will resolve those links and will return
    /// the event that the link points to. Default: [NoResolution](../types/enum.LinkTos.html).
    pub fn resolve_link_tos(self, tos: LinkTos) -> Self {
        let resolve_link_tos = tos.raw_resolve_lnk_tos();

        ReadAllEvents {
            resolve_link_tos,
            ..self
        }
    }

    /// Sends asynchronously the read command to the server.
    pub async fn execute(
        self,
        count: u64,
    ) -> crate::Result<Box<dyn Stream<Item = crate::Result<ResolvedEvent>> + Send + Unpin>> {
        use streams::read_req::options::all_options::AllOption;
        use streams::read_req::options::{self, AllOptions, StreamOption};
        use streams::read_req::Options;

        let read_direction = match self.direction {
            ReadDirection::Forward => 0,
            ReadDirection::Backward => 1,
        };

        let all_option = match self.revision {
            Revision::Exact(pos) => {
                let pos = options::Position {
                    commit_position: pos.commit,
                    prepare_position: pos.prepare,
                };

                AllOption::Position(pos)
            }

            Revision::Start => AllOption::Start(Empty {}),
            Revision::End => AllOption::End(Empty {}),
        };

        let stream_options = AllOptions {
            all_option: Some(all_option),
        };

        let uuid_option = options::UuidOption {
            content: Some(options::uuid_option::Content::String(Empty {})),
        };

        let options = Options {
            stream_option: Some(StreamOption::All(stream_options)),
            resolve_links: self.resolve_link_tos,
            filter_option: Some(options::FilterOption::NoFilter(Empty {})),
            count_option: Some(options::CountOption::Count(count)),
            uuid_option: Some(uuid_option),
            read_direction,
        };

        let req = streams::ReadReq {
            options: Some(options),
        };

        let mut req = Request::new(req);

        configure_auth_req(&mut req, self.creds);

        self.connection
            .execute(|channel| async {
                let mut client = StreamsClient::new(channel);
                let stream = client.read(req).await?.into_inner();
                let stream = stream
                    .try_filter_map(|resp| {
                        let value = match resp.content.unwrap() {
                            streams::read_resp::Content::Event(event) => {
                                Some(convert_proto_read_event(event))
                            }
                            _ => None,
                        };

                        futures::future::ok(value)
                    })
                    .map_err(crate::Error::from_grpc);

                let stream: Box<dyn Stream<Item = crate::Result<ResolvedEvent>> + Send + Unpin> =
                    Box::new(stream);

                Ok(stream)
            })
            .await
    }

    /// Reads all the events of $all stream.
    pub async fn read_through(
        self,
    ) -> crate::Result<Box<dyn Stream<Item = crate::Result<ResolvedEvent>> + Send + Unpin>> {
        self.execute(u64::MAX).await
    }
}

/// Command that deletes a stream. More information on [Deleting stream and events].
///
/// [Deleting stream and events]: https://eventstore.org/docs/server/deleting-streams-and-events/index.html
pub struct DeleteStream {
    connection: GrpcClient,
    stream: String,
    version: ExpectedVersion,
    creds: Option<Credentials>,
    hard_delete: bool,
}

impl DeleteStream {
    pub(crate) fn new(connection: GrpcClient, stream: String, creds: Option<Credentials>) -> Self {
        DeleteStream {
            connection,
            stream,
            hard_delete: false,
            version: ExpectedVersion::Any,
            creds,
        }
    }

    /// Asks the server to check that the stream receiving the event is at
    /// the given expected version. Default: `ExpectedVersion::Any`.
    pub fn expected_version(self, version: ExpectedVersion) -> Self {
        DeleteStream { version, ..self }
    }

    /// Performs the command with the given credentials.
    pub fn credentials(self, value: Credentials) -> Self {
        DeleteStream {
            creds: Some(value),
            ..self
        }
    }

    /// Makes use of Truncate before. When a stream is deleted, its Truncate
    /// before is set to the streams current last event number. When a soft
    /// deleted stream is read, the read will return a StreamNotFound. After
    /// deleting the stream, you are able to write to it again, continuing from
    /// where it left off.
    ///
    /// That is the default behavior.
    pub fn soft_delete(self) -> Self {
        DeleteStream {
            hard_delete: false,
            ..self
        }
    }

    /// A hard delete writes a tombstone event to the stream, permanently
    /// deleting it. The stream cannot be recreated or written to again.
    /// Tombstone events are written with the event type '$streamDeleted'. When
    /// a hard deleted stream is read, the read will return a StreamDeleted.
    pub fn hard_delete(self) -> Self {
        DeleteStream {
            hard_delete: true,
            ..self
        }
    }

    /// Sends asynchronously the delete command to the server.
    pub async fn execute(self) -> crate::Result<Option<Position>> {
        if self.hard_delete {
            use streams::tombstone_req::options::ExpectedStreamRevision;
            use streams::tombstone_req::Options;
            use streams::tombstone_resp::PositionOption;

            let expected_stream_revision = match self.version {
                ExpectedVersion::Any => ExpectedStreamRevision::Any(Empty {}),
                ExpectedVersion::NoStream => ExpectedStreamRevision::NoStream(Empty {}),
                ExpectedVersion::StreamExists => ExpectedStreamRevision::StreamExists(Empty {}),
                ExpectedVersion::Exact(rev) => ExpectedStreamRevision::Revision(rev),
            };

            let expected_stream_revision = Some(expected_stream_revision);
            let stream_identifier = Some(StreamIdentifier {
                stream_name: self.stream.into_bytes(),
            });
            let options = Options {
                stream_identifier,
                expected_stream_revision,
            };

            let mut req = Request::new(streams::TombstoneReq {
                options: Some(options),
            });

            configure_auth_req(&mut req, self.creds);

            self.connection
                .execute(|channel| async {
                    let mut client = StreamsClient::new(channel);
                    let result = client.tombstone(req).await?.into_inner();

                    if let Some(opts) = result.position_option {
                        match opts {
                            PositionOption::Position(pos) => {
                                let pos = Position {
                                    commit: pos.commit_position,
                                    prepare: pos.prepare_position,
                                };

                                Ok(Some(pos))
                            }

                            PositionOption::NoPosition(_) => Ok(None),
                        }
                    } else {
                        Ok(None)
                    }
                })
                .await
        } else {
            use streams::delete_req::options::ExpectedStreamRevision;
            use streams::delete_req::Options;
            use streams::delete_resp::PositionOption;

            let expected_stream_revision = match self.version {
                ExpectedVersion::Any => ExpectedStreamRevision::Any(Empty {}),
                ExpectedVersion::NoStream => ExpectedStreamRevision::NoStream(Empty {}),
                ExpectedVersion::StreamExists => ExpectedStreamRevision::StreamExists(Empty {}),
                ExpectedVersion::Exact(rev) => ExpectedStreamRevision::Revision(rev),
            };

            let expected_stream_revision = Some(expected_stream_revision);
            let stream_identifier = Some(StreamIdentifier {
                stream_name: self.stream.into_bytes(),
            });
            let options = Options {
                stream_identifier,
                expected_stream_revision,
            };

            let mut req = Request::new(streams::DeleteReq {
                options: Some(options),
            });

            configure_auth_req(&mut req, self.creds);

            self.connection
                .execute(|channel| async {
                    let mut client = StreamsClient::new(channel);
                    let result = client.delete(req).await?.into_inner();

                    if let Some(opts) = result.position_option {
                        match opts {
                            PositionOption::Position(pos) => {
                                let pos = Position {
                                    commit: pos.commit_position,
                                    prepare: pos.prepare_position,
                                };

                                Ok(Some(pos))
                            }

                            PositionOption::NoPosition(_) => Ok(None),
                        }
                    } else {
                        Ok(None)
                    }
                })
                .await
        }
    }
}

/// Subscribes to a given stream. This kind of subscription specifies a
/// starting point (by default, the beginning of a stream). For a regular
/// stream, that starting point will be an event number. For the system
/// stream `$all`, it will be a position in the transaction file
/// (see `subscribe_to_all_from`). This subscription will fetch every event
/// until the end of the stream, then will dispatch subsequently written
/// events.
///
/// For example, if a starting point of 50 is specified when a stream has
/// 100 events in it, the subscriber can expect to see events 51 through
/// 100, and then any events subsequenttly written events until such time
/// as the subscription is dropped or closed.
///
/// * Notes
/// Catchup subscription are resilient to connection drops.
/// Basically, if the connection drops. The command will restart its
/// catching up phase from the begining and then emit a new volatile
/// subscription request.
///
/// All this process happens without the user has to do anything.
pub struct RegularCatchupSubscribe {
    connection: GrpcClient,
    stream_id: String,
    resolve_link_tos: bool,
    revision: Option<u64>,
    creds_opt: Option<Credentials>,
}

impl RegularCatchupSubscribe {
    pub(crate) fn new(
        connection: GrpcClient,
        stream_id: String,
        creds_opt: Option<Credentials>,
    ) -> Self {
        RegularCatchupSubscribe {
            connection,
            stream_id,
            resolve_link_tos: false,
            revision: None,
            creds_opt,
        }
    }

    /// When using projections, you can have links placed into another stream.
    /// If you set `true`, the server will resolve those links and will return
    /// the event that the link points to. Default: [NoResolution](../types/enum.LinkTos.html).
    pub fn resolve_link_tos(self, tos: LinkTos) -> Self {
        let resolve_link_tos = tos.raw_resolve_lnk_tos();

        RegularCatchupSubscribe {
            resolve_link_tos,
            ..self
        }
    }

    /// For example, if a starting point of 50 is specified when a stream has
    /// 100 events in it, the subscriber can expect to see events 51 through
    /// 100, and then any events subsequently written events until such time
    /// as the subscription is dropped or closed.
    ///
    /// By default, it will start from the event number 0.
    pub fn start_position(self, start_pos: u64) -> Self {
        let revision = Some(start_pos);
        RegularCatchupSubscribe { revision, ..self }
    }

    /// Performs the command with the given credentials.
    pub fn credentials(self, creds: Credentials) -> Self {
        RegularCatchupSubscribe {
            creds_opt: Some(creds),
            ..self
        }
    }

    /// Runs the subscription command.
    pub async fn execute(
        self,
    ) -> crate::Result<Box<dyn Stream<Item = crate::Result<SubEvent>> + Send + Unpin>> {
        use futures::future;
        use streams::read_req::options::stream_options::RevisionOption;
        use streams::read_req::options::{self, StreamOption, StreamOptions, SubscriptionOptions};
        use streams::read_req::Options;

        let read_direction = 0; // <- Going forward.

        let revision_option = match self.revision {
            Some(rev) => RevisionOption::Revision(rev),
            None => RevisionOption::Start(Empty {}),
        };

        let stream_identifier = Some(StreamIdentifier {
            stream_name: self.stream_id.into_bytes(),
        });
        let stream_options = StreamOptions {
            stream_identifier,
            revision_option: Some(revision_option),
        };

        let uuid_option = options::UuidOption {
            content: Some(options::uuid_option::Content::String(Empty {})),
        };

        let options = Options {
            stream_option: Some(StreamOption::Stream(stream_options)),
            resolve_links: self.resolve_link_tos,
            filter_option: Some(options::FilterOption::NoFilter(Empty {})),
            count_option: Some(options::CountOption::Subscription(SubscriptionOptions {})),
            uuid_option: Some(uuid_option),
            read_direction,
        };

        let req = streams::ReadReq {
            options: Some(options),
        };

        let mut req = Request::new(req);

        configure_auth_req(&mut req, self.creds_opt);

        self.connection
            .execute(|channel| async {
                let mut client = StreamsClient::new(channel);
                let stream = client.read(req).await?.into_inner();
                let stream = stream
                    .try_filter_map(|resp| match resp.content.unwrap() {
                        streams::read_resp::Content::Event(event) => future::ok(Some(
                            SubEvent::EventAppeared(convert_proto_read_event(event)),
                        )),

                        streams::read_resp::Content::Confirmation(sub) => {
                            future::ok(Some(SubEvent::Confirmed(sub.subscription_id)))
                        }

                        _ => future::ok(None),
                    })
                    .map_err(crate::Error::from_grpc);

                let stream: Box<dyn Stream<Item = crate::Result<SubEvent>> + Send + Unpin> =
                    Box::new(stream);

                Ok(stream)
            })
            .await
    }

    /// Runs the subscription command. Filter out all subscription events that are not `SubEvent::EventAppeared`.
    pub async fn execute_event_appeared_only(
        self,
    ) -> crate::Result<Box<dyn Stream<Item = crate::Result<ResolvedEvent>> + Send + Unpin>> {
        let stream = self.execute().await?.try_filter_map(|event| match event {
            SubEvent::EventAppeared(event) => futures::future::ok(Some(event)),
            _ => futures::future::ok(None),
        });

        Ok(Box::new(stream))
    }
}

/// Like `RegularCatchupSubscribe` but specific to the system stream '$all'.
pub struct AllCatchupSubscribe {
    connection: GrpcClient,
    resolve_link_tos: bool,
    revision: Option<Position>,
    creds_opt: Option<Credentials>,
    filter: Option<FilterConf>,
}

impl AllCatchupSubscribe {
    pub(crate) fn new(connection: GrpcClient, creds_opt: Option<Credentials>) -> Self {
        AllCatchupSubscribe {
            connection,
            resolve_link_tos: false,
            revision: None,
            filter: None,
            creds_opt,
        }
    }

    /// When using projections, you can have links placed into another stream.
    /// If you set `true`, the server will resolve those links and will return
    /// the event that the link points to. Default: [NoResolution](../types/enum.LinkTos.html).
    pub fn resolve_link_tos(self, tos: LinkTos) -> Self {
        let resolve_link_tos = tos.raw_resolve_lnk_tos();

        AllCatchupSubscribe {
            resolve_link_tos,
            ..self
        }
    }

    /// Starting point in the transaction journal log. By default, it will start at
    /// `Revision::Start`.
    pub fn start_position(self, start_pos: Position) -> Self {
        let revision = Some(start_pos);

        AllCatchupSubscribe { revision, ..self }
    }

    /// Performs the command with the given credentials.
    pub fn credentials(self, creds: Credentials) -> Self {
        AllCatchupSubscribe {
            creds_opt: Some(creds),
            ..self
        }
    }

    /// Filters events or streams based upon a predicate.
    pub fn filter(self, filter: FilterConf) -> Self {
        AllCatchupSubscribe {
            filter: Some(filter),
            ..self
        }
    }

    /// Preforms the catching up phase of the subscription asynchronously. When
    /// it will reach the head of stream, the command will emit a volatile
    /// subscription request.
    pub async fn execute(
        self,
    ) -> crate::Result<Box<dyn Stream<Item = crate::Result<SubEvent>> + Send + Unpin>> {
        use futures::future;
        use streams::read_req::options::all_options::AllOption;
        use streams::read_req::options::{self, AllOptions, StreamOption, SubscriptionOptions};
        use streams::read_req::Options;

        let read_direction = 0; // <- Going forward.

        let all_option = match self.revision {
            Some(pos) => {
                let pos = options::Position {
                    commit_position: pos.commit,
                    prepare_position: pos.prepare,
                };

                AllOption::Position(pos)
            }

            None => AllOption::Start(Empty {}),
        };

        let stream_options = AllOptions {
            all_option: Some(all_option),
        };

        let uuid_option = options::UuidOption {
            content: Some(options::uuid_option::Content::String(Empty {})),
        };

        let filter_option = match self.filter {
            Some(filter) => options::FilterOption::Filter(filter.into_proto()),
            None => options::FilterOption::NoFilter(Empty {}),
        };

        let options = Options {
            stream_option: Some(StreamOption::All(stream_options)),
            resolve_links: self.resolve_link_tos,
            filter_option: Some(filter_option),
            count_option: Some(options::CountOption::Subscription(SubscriptionOptions {})),
            uuid_option: Some(uuid_option),
            read_direction,
        };

        let req = streams::ReadReq {
            options: Some(options),
        };

        let mut req = Request::new(req);

        configure_auth_req(&mut req, self.creds_opt);

        self.connection
            .execute(|channel| async {
                let mut client = StreamsClient::new(channel);
                let stream = client.read(req).await?.into_inner();
                let stream = stream
                    .try_filter_map(|resp| match resp.content.unwrap() {
                        streams::read_resp::Content::Event(event) => future::ok(Some(
                            SubEvent::EventAppeared(convert_proto_read_event(event)),
                        )),

                        streams::read_resp::Content::Confirmation(sub) => {
                            future::ok(Some(SubEvent::Confirmed(sub.subscription_id)))
                        }

                        streams::read_resp::Content::Checkpoint(chk) => {
                            let position = Position {
                                commit: chk.commit_position,
                                prepare: chk.prepare_position,
                            };

                            future::ok(Some(SubEvent::Checkpoint(position)))
                        }

                        _ => future::ok(None),
                    })
                    .map_err(crate::Error::from_grpc);

                let stream: Box<dyn Stream<Item = crate::Result<SubEvent>> + Send + Unpin> =
                    Box::new(stream);

                Ok(stream)
            })
            .await
    }

    /// Runs the subscription command. Filter out all subscription events that are not `SubEvent::EventAppeared`.
    pub async fn execute_event_appeared_only(
        self,
    ) -> crate::Result<Box<dyn Stream<Item = crate::Result<ResolvedEvent>> + Send + Unpin>> {
        let stream = self.execute().await?.try_filter_map(|event| match event {
            SubEvent::EventAppeared(event) => futures::future::ok(Some(event)),
            _ => futures::future::ok(None),
        });

        Ok(Box::new(stream))
    }
}

/// A command that creates a persistent subscription for a given group.
pub struct CreatePersistentSubscription {
    connection: GrpcClient,
    stream_id: String,
    group_name: String,
    creds: Option<Credentials>,
}

impl CreatePersistentSubscription {
    pub(crate) fn new(
        connection: GrpcClient,
        stream_id: String,
        group_name: String,
        creds: Option<Credentials>,
    ) -> Self {
        CreatePersistentSubscription {
            connection,
            stream_id,
            group_name,
            creds,
        }
    }

    /// Performs the command with the given credentials.
    pub fn credentials(self, creds: Credentials) -> Self {
        CreatePersistentSubscription {
            creds: Some(creds),
            ..self
        }
    }

    /// Sends the persistent subscription creation command asynchronously to
    /// the server.
    pub async fn execute(self, settings: PersistentSubscriptionSettings) -> crate::Result<()> {
        use persistent::create_req::Options;
        use persistent::CreateReq;

        let settings = convert_settings_create(settings);
        let stream_identifier = Some(StreamIdentifier {
            stream_name: self.stream_id.into_bytes(),
        });
        let options = Options {
            stream_identifier,
            group_name: self.group_name,
            settings: Some(settings),
        };

        let req = CreateReq {
            options: Some(options),
        };

        let mut req = Request::new(req);

        configure_auth_req(&mut req, self.creds);

        self.connection
            .execute(|channel| async {
                let mut client = PersistentSubscriptionsClient::new(channel);
                client.create(req).await?;

                Ok(())
            })
            .await
    }
}

/// Command that updates an already existing subscription's settings.
pub struct UpdatePersistentSubscription {
    connection: GrpcClient,
    stream_id: String,
    group_name: String,
    creds: Option<Credentials>,
}

impl UpdatePersistentSubscription {
    pub(crate) fn new(
        connection: GrpcClient,
        stream_id: String,
        group_name: String,
        creds: Option<Credentials>,
    ) -> Self {
        UpdatePersistentSubscription {
            connection,
            stream_id,
            group_name,
            creds,
        }
    }

    /// Performs the command with the given credentials.
    pub fn credentials(self, creds: Credentials) -> Self {
        UpdatePersistentSubscription {
            creds: Some(creds),
            ..self
        }
    }

    /// Sends the persistent subscription update command asynchronously to
    /// the server.
    pub async fn execute(self, settings: PersistentSubscriptionSettings) -> crate::Result<()> {
        use persistent::update_req::Options;
        use persistent::UpdateReq;

        let settings = convert_settings_update(settings);
        let stream_identifier = Some(StreamIdentifier {
            stream_name: self.stream_id.into_bytes(),
        });
        let options = Options {
            stream_identifier,
            group_name: self.group_name,
            settings: Some(settings),
        };

        let req = UpdateReq {
            options: Some(options),
        };

        let mut req = Request::new(req);

        configure_auth_req(&mut req, self.creds);

        self.connection
            .execute(|channel| async {
                let mut client = PersistentSubscriptionsClient::new(channel);
                client.update(req).await?;

                Ok(())
            })
            .await
    }
}

/// Command that  deletes a persistent subscription.
pub struct DeletePersistentSubscription {
    connection: GrpcClient,
    stream_id: String,
    group_name: String,
    creds: Option<Credentials>,
}

impl DeletePersistentSubscription {
    pub(crate) fn new(
        connection: GrpcClient,
        stream_id: String,
        group_name: String,
        creds: Option<Credentials>,
    ) -> Self {
        DeletePersistentSubscription {
            connection,
            stream_id,
            group_name,
            creds,
        }
    }

    /// Performs the command with the given credentials.
    pub fn credentials(self, creds: Credentials) -> Self {
        DeletePersistentSubscription {
            creds: Some(creds),
            ..self
        }
    }

    /// Sends the persistent subscription deletion command asynchronously to
    /// the server.
    pub async fn execute(self) -> crate::Result<()> {
        use persistent::delete_req::Options;

        let stream_identifier = Some(StreamIdentifier {
            stream_name: self.stream_id.into_bytes(),
        });
        let options = Options {
            stream_identifier,
            group_name: self.group_name,
        };

        let req = persistent::DeleteReq {
            options: Some(options),
        };

        let mut req = Request::new(req);

        configure_auth_req(&mut req, self.creds);

        self.connection
            .execute(|channel| async {
                let mut client = PersistentSubscriptionsClient::new(channel);
                client.delete(req).await?;

                Ok(())
            })
            .await
    }
}

/// A subscription model where the server remembers the state of the
/// consumption of a stream. This allows for many different modes of operations
/// compared to a regular subscription where the client hols the subscription
/// state.
pub struct ConnectToPersistentSubscription {
    connection: GrpcClient,
    stream_id: String,
    group_name: String,
    batch_size: i32,
    creds: Option<Credentials>,
}

impl ConnectToPersistentSubscription {
    pub(crate) fn new(
        connection: GrpcClient,
        stream_id: String,
        group_name: String,
        creds: Option<Credentials>,
    ) -> Self {
        ConnectToPersistentSubscription {
            connection,
            stream_id,
            group_name,
            batch_size: 10,
            creds,
        }
    }

    /// Performs the command with the given credentials.
    pub fn credentials(self, creds: Credentials) -> Self {
        ConnectToPersistentSubscription {
            creds: Some(creds),
            ..self
        }
    }

    /// The buffer size to use  for the persistent subscription.
    pub fn batch_size(self, batch_size: i32) -> Self {
        ConnectToPersistentSubscription { batch_size, ..self }
    }

    /// Sends the persistent subscription connection request to the server
    /// asynchronously even if the subscription is available right away.
    pub async fn execute(self) -> crate::Result<(SubscriptionRead, SubscriptionWrite)> {
        use futures::channel::mpsc;
        use futures::sink::SinkExt;
        use persistent::read_req::options::{self, UuidOption};
        use persistent::read_req::{self, Options};
        use persistent::read_resp;
        use persistent::ReadReq;

        let (mut sender, recv) = mpsc::channel(500);

        let uuid_option = UuidOption {
            content: Some(options::uuid_option::Content::String(Empty {})),
        };

        let stream_identifier = Some(StreamIdentifier {
            stream_name: self.stream_id.into_bytes(),
        });
        let options = Options {
            stream_identifier,
            group_name: self.group_name,
            buffer_size: self.batch_size,
            uuid_option: Some(uuid_option),
        };

        let read_req = ReadReq {
            content: Some(read_req::Content::Options(options)),
        };

        let mut req = Request::new(recv);

        configure_auth_req(&mut req, self.creds.clone());

        let _ = sender.send(read_req).await;

        self.connection
            .execute(|channel| async {
                let mut client = PersistentSubscriptionsClient::new(channel);
                let stream = client.read(req).await?.into_inner();

                let stream = stream
                    .try_filter_map(|resp| {
                        let ret = match resp
                            .content
                            .expect("Why response content wouldn't be defined?")
                        {
                            read_resp::Content::Event(evt) => Some(SubEvent::EventAppeared(
                                convert_persistent_proto_read_event(evt),
                            )),

                            read_resp::Content::SubscriptionConfirmation(sub) => {
                                Some(SubEvent::Confirmed(sub.subscription_id))
                            }
                        };

                        futures::future::ready(Ok(ret))
                    })
                    .map_err(crate::Error::from_grpc);

                let read = SubscriptionRead {
                    inner: Box::new(stream),
                };
                let write = SubscriptionWrite { sender };

                Ok((read, write))
            })
            .await
    }
}

pub struct SubscriptionRead {
    inner: Box<dyn Stream<Item = crate::Result<SubEvent>> + Send + Unpin>,
}

impl SubscriptionRead {
    pub async fn try_next(&mut self) -> crate::Result<Option<SubEvent>> {
        self.inner.try_next().await
    }

    pub async fn try_next_event(&mut self) -> crate::Result<Option<ResolvedEvent>> {
        let event = self.inner.try_next().await?;

        if let Some(event) = event {
            if let SubEvent::EventAppeared(event) = event {
                return Ok(Some(event));
            }
        }

        Ok(None)
    }
}
fn to_proto_uuid(id: uuid::Uuid) -> Uuid {
    Uuid {
        value: Some(shared::uuid::Value::String(format!("{}", id))),
    }
}

pub struct SubscriptionWrite {
    sender: futures::channel::mpsc::Sender<persistent::ReadReq>,
}

impl SubscriptionWrite {
    pub async fn ack_event(&mut self, event: ResolvedEvent) -> Result<(), tonic::Status> {
        self.ack(vec![event.get_original_event().id]).await
    }

    pub async fn ack<I>(&mut self, event_ids: I) -> Result<(), tonic::Status>
    where
        I: IntoIterator<Item = uuid::Uuid>,
    {
        use futures::sink::SinkExt;
        use persistent::read_req::{Ack, Content};
        use persistent::ReadReq;

        let ids = event_ids.into_iter().map(to_proto_uuid).collect();
        let ack = Ack {
            id: Vec::new(),
            ids,
        };

        let content = Content::Ack(ack);
        let read_req = ReadReq {
            content: Some(content),
        };

        let _ = self.sender.send(read_req).await;

        Ok(())
    }

    pub async fn nack<I>(
        &mut self,
        event_ids: I,
        action: NakAction,
        reason: String,
    ) -> Result<(), tonic::Status>
    where
        I: Iterator<Item = uuid::Uuid>,
    {
        use futures::sink::SinkExt;
        use persistent::read_req::{Content, Nack};
        use persistent::ReadReq;

        let ids = event_ids.map(to_proto_uuid).collect();

        let action = match action {
            NakAction::Unknown => 0,
            NakAction::Park => 1,
            NakAction::Retry => 2,
            NakAction::Skip => 3,
            NakAction::Stop => 4,
        };

        let nack = Nack {
            id: Vec::new(),
            ids,
            action,
            reason,
        };

        let content = Content::Nack(nack);
        let read_req = ReadReq {
            content: Some(content),
        };

        let _ = self.sender.send(read_req).await;

        Ok(())
    }
}
