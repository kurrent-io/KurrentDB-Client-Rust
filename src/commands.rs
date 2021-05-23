//! Commands this client supports.
use futures::{stream, TryStreamExt};
use futures::{Stream, StreamExt};

use crate::event_store::client::{persistent, shared, streams};
use crate::types::{
    EventData, ExpectedRevision, PersistentSubscriptionSettings, Position, ReadDirection,
    RecordedEvent, ResolvedEvent, StreamPosition, SubEvent, WriteResult, WrongExpectedVersion,
};

use async_stream::stream;
use persistent::persistent_subscriptions_client::PersistentSubscriptionsClient;
use shared::{Empty, StreamIdentifier, Uuid};
use streams::streams_client::StreamsClient;

use crate::grpc::GrpcClient;
use crate::options::append_to_stream::AppendToStreamOptions;
use crate::options::persistent_subscription::PersistentSubscriptionOptions;
use crate::options::read_all::ReadAllOptions;
use crate::options::read_stream::ReadStreamOptions;
use crate::options::subscribe_to_stream::SubscribeToStreamOptions;
use crate::{
    ConnectToPersistentSubscription, Credentials, CurrentRevision,
    DeletePersistentSubscriptionOptions, DeleteStreamOptions, NakAction, ReadResult,
    SubscribeToAllOptions, SubscriptionFilter, SystemConsumerStrategy,
};
use futures::stream::BoxStream;
use tonic::Request;

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

fn convert_event_data_to_batch_proposed_message(
    event: EventData,
) -> streams::batch_append_req::ProposedMessage {
    use streams::batch_append_req;

    let id = event.id_opt.unwrap_or_else(uuid::Uuid::new_v4);
    let id = shared::uuid::Value::String(id.to_string());
    let id = Uuid { value: Some(id) };
    let custom_metadata = event
        .custom_metadata
        .map_or_else(Vec::new, |b| (&*b).into());

    batch_append_req::ProposedMessage {
        id: Some(id),
        metadata: event.metadata,
        custom_metadata,
        data: (&*event.payload).into(),
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
        resolve_links: settings.resolve_link_tos,
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
        resolve_links: settings.resolve_link_tos,
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
pub fn filter_into_proto(filter: SubscriptionFilter) -> streams::read_req::options::FilterOptions {
    use options::filter_options::{Expression, Filter, Window};
    use streams::read_req::options::{self, FilterOptions};

    let window = match filter.max {
        Some(max) => Window::Max(max),
        None => Window::Count(Empty {}),
    };

    let expr = Expression {
        regex: filter.regex.unwrap_or_else(|| "".to_string()),
        prefix: filter.prefixes,
    };

    let filter = if filter.based_on_stream {
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

/// Sends asynchronously the write command to the server.
pub async fn append_to_stream<S, Events>(
    connection: &GrpcClient,
    stream: S,
    options: &AppendToStreamOptions,
    events: Events,
) -> crate::Result<Result<WriteResult, WrongExpectedVersion>>
where
    S: AsRef<str>,
    Events: Stream<Item = EventData> + Send + Sync + 'static,
{
    use streams::append_req::{self, Content};
    use streams::AppendReq;

    let stream = stream.as_ref().to_string();

    connection.execute(move |channel| async move {
        let stream_identifier = Some(StreamIdentifier {
            stream_name: stream.into_bytes(),
        });
        let header = Content::Options(append_req::Options {
            stream_identifier,
            expected_stream_revision: Some(options.version.clone()),
        });
        let header = AppendReq {
            content: Some(header),
        };
        let header = stream::once(async move { header });
        let events = events.map(convert_event_data);
        let payload = header.chain(events);
        let mut req = Request::new(payload);

        let credentials = options.credentials.clone().or_else(|| connection.default_credentials());

        configure_auth_req(&mut req, credentials);

        let mut client = StreamsClient::new(channel.channel);
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
                    streams::append_resp::wrong_expected_version::CurrentRevisionOption::CurrentNoStream(_) => CurrentRevision::NoStream,
                };

                let expected = match error.expected_revision_option.unwrap() {
                    streams::append_resp::wrong_expected_version::ExpectedRevisionOption::ExpectedRevision(rev) => ExpectedRevision::Exact(rev),
                    streams::append_resp::wrong_expected_version::ExpectedRevisionOption::ExpectedAny(_) => ExpectedRevision::Any,
                    streams::append_resp::wrong_expected_version::ExpectedRevisionOption::ExpectedStreamExists(_) => ExpectedRevision::StreamExists,
                    streams::append_resp::wrong_expected_version::ExpectedRevisionOption::ExpectedNoStream(_) => ExpectedRevision::NoStream,
                };

                Ok(Err(WrongExpectedVersion { current, expected }))
            }
        }
    }).await
}

pub async fn batch_append<'a, Events>(
    connection: &GrpcClient,
    credentials: Option<&crate::types::Credentials>,
    mut events: Events,
) -> crate::Result<BoxStream<'a, crate::Result<crate::types::BatchResp>>>
where
    Events: Stream<Item = crate::types::Batch> + Send + Sync + Unpin + 'static,
{
    use streams::{
        batch_append_req::{options::ExpectedStreamPosition, Options, ProposedMessage},
        batch_append_resp::{
            self,
            success::{CurrentRevisionOption, PositionOption},
        },
        BatchAppendReq,
    };

    let stream = async_stream::stream! {
        while let Some(batch) = events.next().await {
            let correlation_id = shared::uuid::Value::String(batch.correlation_id.to_string());
            let correlation_id = Some(Uuid { value: Some(correlation_id) });
            let stream_identifier = Some(StreamIdentifier {
                stream_name: batch.stream_name.into_bytes(),
            });

            let expected_stream_position = match batch.expected_version {
                ExpectedRevision::Exact(rev) => ExpectedStreamPosition::StreamPosition(rev),
                ExpectedRevision::NoStream => ExpectedStreamPosition::NoStream(()),
                ExpectedRevision::StreamExists => ExpectedStreamPosition::StreamExists(()),
                ExpectedRevision::Any => ExpectedStreamPosition::Any(()),
            };

            let proposed_messages: Vec<ProposedMessage> = batch.events.into_iter().map(convert_event_data_to_batch_proposed_message).collect();

            let expected_stream_position = Some(expected_stream_position);

            let options = Some(Options {
                stream_identifier,
                deadline: None,
                expected_stream_position,
            });

            yield (BatchAppendReq {
                correlation_id,
                options,
                proposed_messages,
                is_final: batch.is_final,
            });
        };
    };

    let credentials = credentials
        .cloned()
        .or_else(|| connection.default_credentials());

    let mut resp_stream = connection
        .execute(move |handle| async move {
            let mut req = Request::new(stream);
            configure_auth_req(&mut req, credentials);
            let mut client = StreamsClient::new(handle.channel.clone());

            let resp = client.batch_append(req).await?;

            Ok(resp.into_inner())
        })
        .await?;

    let resp_stream = resp_stream
        .map_err(crate::types::Error::from_grpc)
        .map_ok(|resp| {
            let stream_name = String::from_utf8(resp.stream_identifier.unwrap().stream_name)
                .expect("valid UTF-8 string");

            let correlation_id = raw_uuid_to_uuid(resp.correlation_id.unwrap());

            let result = match resp.result.unwrap() {
                batch_append_resp::Result::Success(success) => {
                    let result = crate::types::BatchWriteResult {
                        current_revision: match success.current_revision_option.unwrap() {
                            CurrentRevisionOption::CurrentRevision(rev) => Some(rev),
                            CurrentRevisionOption::NoStream(()) => None,
                        },
                        position: match success.position_option.unwrap() {
                            PositionOption::Position(pos) => Some(Position {
                                commit: pos.commit_position,
                                prepare: pos.prepare_position,
                            }),
                            PositionOption::NoPosition(_) => None,
                        },
                    };

                    Ok(result)
                }
                batch_append_resp::Result::Error(code) => Err(code),
            };

            let expected_version = match resp.expected_stream_position.unwrap() {
                batch_append_resp::ExpectedStreamPosition::Any(_) => {
                    crate::types::ExpectedRevision::Any
                }
                batch_append_resp::ExpectedStreamPosition::NoStream(_) => {
                    crate::types::ExpectedRevision::NoStream
                }
                batch_append_resp::ExpectedStreamPosition::StreamExists(_) => {
                    crate::types::ExpectedRevision::StreamExists
                }
                batch_append_resp::ExpectedStreamPosition::StreamPosition(rev) => {
                    crate::types::ExpectedRevision::Exact(rev)
                }
            };

            crate::types::BatchResp {
                stream_name,
                correlation_id,
                result,
                expected_version,
            }
        });

    Ok(resp_stream.boxed())
}

/// Sends asynchronously the read command to the server.
pub async fn read_stream<'a, S: AsRef<str>>(
    connection: &GrpcClient,
    options: &ReadStreamOptions,
    stream: S,
    count: u64,
) -> crate::Result<ReadResult<BoxStream<'a, crate::Result<ResolvedEvent>>>> {
    use streams::read_req::options::stream_options::RevisionOption;
    use streams::read_req::options::{self, StreamOption, StreamOptions};
    use streams::read_req::Options;

    let read_direction = match options.direction {
        ReadDirection::Forward => 0,
        ReadDirection::Backward => 1,
    };

    let revision_option = match options.position {
        StreamPosition::Position(rev) => RevisionOption::Revision(rev),
        StreamPosition::Start => RevisionOption::Start(Empty {}),
        StreamPosition::End => RevisionOption::End(Empty {}),
    };

    let stream_identifier = Some(StreamIdentifier {
        stream_name: stream.as_ref().to_string().into_bytes(),
    });
    let stream_options = StreamOptions {
        stream_identifier,
        revision_option: Some(revision_option),
    };

    let uuid_option = options::UuidOption {
        content: Some(options::uuid_option::Content::String(Empty {})),
    };

    let credentials = options
        .credentials
        .clone()
        .or_else(|| connection.default_credentials());

    let options = Options {
        stream_option: Some(StreamOption::Stream(stream_options)),
        resolve_links: options.resolve_link_tos,
        filter_option: Some(options::FilterOption::NoFilter(Empty {})),
        count_option: Some(options::CountOption::Count(count)),
        uuid_option: Some(uuid_option),
        read_direction,
    };

    let req = streams::ReadReq {
        options: Some(options),
    };

    let mut req = Request::new(req);

    configure_auth_req(&mut req, credentials);

    connection
        .execute(|channel| async {
            let mut client = StreamsClient::new(channel.channel.clone());
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
                        let stream = stream! {
                            // We send back to the user the first event we received.
                            if let streams::read_resp::Content::Event(event) = resp.content.expect("content is defined") {
                                yield Ok(convert_proto_read_event(event));
                            }

                            loop {
                                match stream.try_next().await {
                                    Err(e) => {
                                        let e = crate::Error::from_grpc(e);

                                        channel.report_error(e.clone()).await;
                                        yield Err(e);
                                        break;
                                    }

                                    Ok(resp) => {
                                        if let Some(resp) = resp {
                                            if let streams::read_resp::Content::Event(event) = resp.content.expect("content is defined") {
                                                yield Ok(convert_proto_read_event(event));
                                            }

                                            continue;
                                        }

                                        break;
                                    }
                                }
                            }
                        };

                        let stream: BoxStream<crate::Result<ResolvedEvent>> = Box::pin(stream);

                        return Ok(ReadResult::Ok(stream));
                    }
                }
            }

            Ok(ReadResult::Ok(Box::pin(stream::empty())))
        })
        .await
}

pub async fn read_all<'a>(
    connection: &GrpcClient,
    options: &ReadAllOptions,
    count: u64,
) -> crate::Result<BoxStream<'a, crate::Result<ResolvedEvent>>> {
    use streams::read_req::options::all_options::AllOption;
    use streams::read_req::options::{self, AllOptions, StreamOption};
    use streams::read_req::Options;

    let read_direction = match options.direction {
        ReadDirection::Forward => 0,
        ReadDirection::Backward => 1,
    };

    let all_option = match options.position {
        StreamPosition::Position(pos) => {
            let pos = options::Position {
                commit_position: pos.commit,
                prepare_position: pos.prepare,
            };

            AllOption::Position(pos)
        }

        StreamPosition::Start => AllOption::Start(Empty {}),
        StreamPosition::End => AllOption::End(Empty {}),
    };

    let stream_options = AllOptions {
        all_option: Some(all_option),
    };

    let uuid_option = options::UuidOption {
        content: Some(options::uuid_option::Content::String(Empty {})),
    };

    let credentials = options
        .credentials
        .clone()
        .or_else(|| connection.default_credentials());

    let options = Options {
        stream_option: Some(StreamOption::All(stream_options)),
        resolve_links: options.resolve_link_tos,
        filter_option: Some(options::FilterOption::NoFilter(Empty {})),
        count_option: Some(options::CountOption::Count(count)),
        uuid_option: Some(uuid_option),
        read_direction,
    };

    let req = streams::ReadReq {
        options: Some(options),
    };

    let mut req = Request::new(req);

    configure_auth_req(&mut req, credentials);

    connection
        .execute(|channel| async {
            let mut client = StreamsClient::new(channel.channel.clone());
            let mut stream = client.read(req).await?.into_inner();

            let stream = stream! {
                loop {
                    match stream.try_next().await {
                        Err(e) => {
                            let e = crate::Error::from_grpc(e);

                            channel.report_error(e.clone()).await;
                            yield Err(e);
                            break;
                        }

                        Ok(resp) => {
                            if let Some(resp) = resp {
                                if let streams::read_resp::Content::Event(event) = resp.content.expect("content is defined") {
                                    yield Ok(convert_proto_read_event(event));
                                }

                                continue;
                            }

                            break;
                        }
                    }
                }
            };

            let stream: BoxStream<crate::Result<ResolvedEvent>> = Box::pin(stream);

            Ok(stream)
        })
        .await
}

/// Sends asynchronously the delete command to the server.
pub async fn delete_stream<S: AsRef<str>>(
    connection: &GrpcClient,
    stream: S,
    options: &DeleteStreamOptions,
) -> crate::Result<Option<Position>> {
    let credentials = options
        .credentials
        .clone()
        .or_else(|| connection.default_credentials());

    if options.hard_delete {
        use streams::tombstone_req::options::ExpectedStreamRevision;
        use streams::tombstone_req::Options;
        use streams::tombstone_resp::PositionOption;

        let expected_stream_revision = match options.version {
            ExpectedRevision::Any => ExpectedStreamRevision::Any(Empty {}),
            ExpectedRevision::NoStream => ExpectedStreamRevision::NoStream(Empty {}),
            ExpectedRevision::StreamExists => ExpectedStreamRevision::StreamExists(Empty {}),
            ExpectedRevision::Exact(rev) => ExpectedStreamRevision::Revision(rev),
        };

        let expected_stream_revision = Some(expected_stream_revision);
        let stream_identifier = Some(StreamIdentifier {
            stream_name: stream.as_ref().to_string().into_bytes(),
        });
        let options = Options {
            stream_identifier,
            expected_stream_revision,
        };

        let mut req = Request::new(streams::TombstoneReq {
            options: Some(options),
        });

        configure_auth_req(&mut req, credentials);

        connection
            .execute(|channel| async {
                let mut client = StreamsClient::new(channel.channel);
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

        let expected_stream_revision = match options.version {
            ExpectedRevision::Any => ExpectedStreamRevision::Any(Empty {}),
            ExpectedRevision::NoStream => ExpectedStreamRevision::NoStream(Empty {}),
            ExpectedRevision::StreamExists => ExpectedStreamRevision::StreamExists(Empty {}),
            ExpectedRevision::Exact(rev) => ExpectedStreamRevision::Revision(rev),
        };

        let expected_stream_revision = Some(expected_stream_revision);
        let stream_identifier = Some(StreamIdentifier {
            stream_name: stream.as_ref().to_string().into_bytes(),
        });
        let options = Options {
            stream_identifier,
            expected_stream_revision,
        };

        let mut req = Request::new(streams::DeleteReq {
            options: Some(options),
        });

        configure_auth_req(&mut req, credentials);

        connection
            .execute(|channel| async {
                let mut client = StreamsClient::new(channel.channel);
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

/// Runs the subscription command.
pub async fn subscribe_to_stream<'a, S: AsRef<str>>(
    connection: &GrpcClient,
    stream_id: S,
    options: &SubscribeToStreamOptions,
) -> crate::Result<BoxStream<'a, crate::Result<SubEvent>>> {
    use streams::read_req::options::stream_options::RevisionOption;
    use streams::read_req::options::{self, StreamOption, StreamOptions, SubscriptionOptions};
    use streams::read_req::Options;

    let read_direction = 0; // <- Going forward.

    let revision = match options.position {
        StreamPosition::Start => RevisionOption::Start(Empty {}),
        StreamPosition::End => RevisionOption::End(Empty {}),
        StreamPosition::Position(revision) => RevisionOption::Revision(revision),
    };

    let stream_identifier = Some(StreamIdentifier {
        stream_name: stream_id.as_ref().to_string().into_bytes(),
    });
    let stream_options = StreamOptions {
        stream_identifier,
        revision_option: Some(revision),
    };

    let uuid_option = options::UuidOption {
        content: Some(options::uuid_option::Content::String(Empty {})),
    };

    let credentials = options
        .credentials
        .clone()
        .or_else(|| connection.default_credentials());

    let options = Options {
        stream_option: Some(StreamOption::Stream(stream_options)),
        resolve_links: options.resolve_link_tos,
        filter_option: Some(options::FilterOption::NoFilter(Empty {})),
        count_option: Some(options::CountOption::Subscription(SubscriptionOptions {})),
        uuid_option: Some(uuid_option),
        read_direction,
    };

    let req = streams::ReadReq {
        options: Some(options),
    };

    let mut req = Request::new(req);

    configure_auth_req(&mut req, credentials);

    connection
        .execute(|channel| async {
            let mut client = StreamsClient::new(channel.channel.clone());
            let mut stream = client.read(req).await?.into_inner();

            let stream = stream! {
                loop {
                    match stream.try_next().await {
                        Err(e) => {
                            let e = crate::Error::from_grpc(e);

                            channel.report_error(e.clone()).await;
                            yield Err(e);
                            break;
                        }

                        Ok(resp) => {
                            if let Some(resp) = resp {
                                match resp.content.expect("content is defined") {
                                    streams::read_resp::Content::Event(event) => {
                                        yield Ok(SubEvent::EventAppeared(convert_proto_read_event(event)));
                                    }

                                    streams::read_resp::Content::Confirmation(sub) => {
                                        yield Ok(SubEvent::Confirmed(sub.subscription_id));
                                    }

                                    _ => {}
                                }
                                continue;
                            }

                            break;
                        }
                    }
                }
            };

            let stream: BoxStream<crate::Result<SubEvent>> = Box::pin(stream);

            Ok(stream)
        })
        .await
}

pub async fn subscribe_to_all<'a>(
    connection: &GrpcClient,
    options: &SubscribeToAllOptions,
) -> crate::Result<BoxStream<'a, crate::Result<SubEvent>>> {
    use streams::read_req::options::all_options::AllOption;
    use streams::read_req::options::{self, AllOptions, StreamOption, SubscriptionOptions};
    use streams::read_req::Options;

    let read_direction = 0; // <- Going forward.

    let revision = match options.position {
        StreamPosition::Start => AllOption::Start(Empty {}),
        StreamPosition::Position(pos) => AllOption::Position(options::Position {
            prepare_position: pos.prepare,
            commit_position: pos.commit,
        }),
        StreamPosition::End => AllOption::End(Empty {}),
    };

    let stream_options = AllOptions {
        all_option: Some(revision),
    };

    let uuid_option = options::UuidOption {
        content: Some(options::uuid_option::Content::String(Empty {})),
    };

    let filter_option = match options.filter.as_ref() {
        Some(filter) => options::FilterOption::Filter(filter_into_proto(filter.clone())),
        None => options::FilterOption::NoFilter(Empty {}),
    };

    let credentials = options
        .credentials
        .clone()
        .or_else(|| connection.default_credentials());

    let options = Options {
        stream_option: Some(StreamOption::All(stream_options)),
        resolve_links: options.resolve_link_tos,
        filter_option: Some(filter_option),
        count_option: Some(options::CountOption::Subscription(SubscriptionOptions {})),
        uuid_option: Some(uuid_option),
        read_direction,
    };

    let req = streams::ReadReq {
        options: Some(options),
    };

    let mut req = Request::new(req);

    configure_auth_req(&mut req, credentials);

    connection
        .execute(|channel| async {
            let mut client = StreamsClient::new(channel.channel.clone());
            let mut stream = client.read(req).await?.into_inner();

            let stream = stream! {
                loop {
                    match stream.try_next().await {
                        Err(e) => {
                            let e = crate::Error::from_grpc(e);

                            channel.report_error(e.clone()).await;
                            yield Err(e);
                            break;
                        }

                        Ok(resp) => {
                            if let Some(resp) = resp {
                                match resp.content.expect("content is defined") {
                                    streams::read_resp::Content::Event(event) => {
                                        yield Ok(SubEvent::EventAppeared(convert_proto_read_event(event)));
                                    }

                                    streams::read_resp::Content::Confirmation(sub) => {
                                        yield Ok(SubEvent::Confirmed(sub.subscription_id));
                                    }

                                    streams::read_resp::Content::Checkpoint(chk) => {
                                        let position = Position {
                                            commit: chk.commit_position,
                                            prepare: chk.prepare_position,
                                        };

                                        yield Ok(SubEvent::Checkpoint(position));
                                    }

                                    _ => {}
                                }
                                continue;
                            }

                            break;
                        }
                    }
                }
            };

            let stream: BoxStream<'a, crate::Result<SubEvent>> = Box::pin(stream);

            Ok(stream)
        })
        .await
}

pub async fn create_persistent_subscription<S: AsRef<str>>(
    connection: &GrpcClient,
    stream: S,
    group: S,
    options: &PersistentSubscriptionOptions,
) -> crate::Result<()> {
    use persistent::create_req::{
        options::StreamOption, stream_options::RevisionOption, Options, StreamOptions,
    };
    use persistent::CreateReq;

    let settings = convert_settings_create(options.setts);
    let stream_identifier = Some(StreamIdentifier {
        stream_name: stream.as_ref().to_string().into_bytes(),
    });

    let credentials = options
        .credentials
        .clone()
        .or_else(|| connection.default_credentials());

    let revision_option = match options.revision {
        StreamPosition::Start => RevisionOption::Start(Empty {}),
        StreamPosition::End => RevisionOption::End(Empty {}),
        StreamPosition::Position(rev) => RevisionOption::Revision(rev),
    };

    let revision_option = Some(revision_option);

    let stream_option = StreamOptions {
        stream_identifier,
        revision_option,
    };

    let stream_option = StreamOption::Stream(stream_option);
    let stream_option = Some(stream_option);

    #[allow(deprecated)]
    let options = Options {
        stream_identifier: None,
        group_name: group.as_ref().to_string(),
        settings: Some(settings),
        stream_option,
    };

    let req = CreateReq {
        options: Some(options),
    };

    let mut req = Request::new(req);

    configure_auth_req(&mut req, credentials);

    connection
        .execute(|channel| async {
            let mut client = PersistentSubscriptionsClient::new(channel.channel);
            client.create(req).await?;

            Ok(())
        })
        .await
}

pub async fn update_persistent_subscription<S: AsRef<str>>(
    connection: &GrpcClient,
    stream: S,
    group: S,
    options: &PersistentSubscriptionOptions,
) -> crate::Result<()> {
    use persistent::update_req::{
        options::StreamOption, stream_options::RevisionOption, Options, StreamOptions,
    };
    use persistent::UpdateReq;

    let settings = convert_settings_update(options.setts);
    let stream_identifier = Some(StreamIdentifier {
        stream_name: stream.as_ref().to_string().into_bytes(),
    });

    let credentials = options
        .credentials
        .clone()
        .or_else(|| connection.default_credentials());

    let revision_option = match options.revision {
        StreamPosition::Start => RevisionOption::Start(Empty {}),
        StreamPosition::End => RevisionOption::End(Empty {}),
        StreamPosition::Position(rev) => RevisionOption::Revision(rev),
    };

    let revision_option = Some(revision_option);

    let stream_option = StreamOptions {
        stream_identifier,
        revision_option,
    };

    let stream_option = StreamOption::Stream(stream_option);
    let stream_option = Some(stream_option);

    #[allow(deprecated)]
    let options = Options {
        stream_identifier: None,
        group_name: group.as_ref().to_string(),
        settings: Some(settings),
        stream_option,
    };

    let req = UpdateReq {
        options: Some(options),
    };

    let mut req = Request::new(req);

    configure_auth_req(&mut req, credentials);

    connection
        .execute(|channel| async {
            let mut client = PersistentSubscriptionsClient::new(channel.channel);
            client.update(req).await?;

            Ok(())
        })
        .await
}

pub async fn delete_persistent_subscription<S: AsRef<str>>(
    connection: &GrpcClient,
    stream_id: S,
    group_name: S,
    options: &DeletePersistentSubscriptionOptions,
) -> crate::Result<()> {
    use persistent::delete_req::{options::StreamOption, Options};

    let stream_identifier = StreamIdentifier {
        stream_name: stream_id.as_ref().to_string().into_bytes(),
    };

    let credentials = options
        .credentials
        .clone()
        .or_else(|| connection.default_credentials());

    let stream_option = StreamOption::StreamIdentifier(stream_identifier);
    let stream_option = Some(stream_option);

    #[allow(deprecated)]
    let options = Options {
        group_name: group_name.as_ref().to_string(),
        stream_option,
    };

    let req = persistent::DeleteReq {
        options: Some(options),
    };

    let mut req = Request::new(req);

    configure_auth_req(&mut req, credentials);

    connection
        .execute(|channel| async {
            let mut client = PersistentSubscriptionsClient::new(channel.channel);
            client.delete(req).await?;

            Ok(())
        })
        .await
}

/// Sends the persistent subscription connection request to the server
/// asynchronously even if the subscription is available right away.
pub async fn connect_persistent_subscription<S: AsRef<str>>(
    connection: &GrpcClient,
    stream_id: S,
    group_name: S,
    options: &ConnectToPersistentSubscription,
) -> crate::Result<(SubscriptionRead, SubscriptionWrite)> {
    use futures::channel::mpsc;
    use futures::sink::SinkExt;
    use persistent::read_req::options::{self, StreamOption, UuidOption};
    use persistent::read_req::{self, Options};
    use persistent::read_resp;
    use persistent::ReadReq;

    let (mut sender, recv) = mpsc::channel(500);

    let uuid_option = UuidOption {
        content: Some(options::uuid_option::Content::String(Empty {})),
    };

    let stream_identifier = StreamIdentifier {
        stream_name: stream_id.as_ref().to_string().into_bytes(),
    };

    let credentials = options
        .credentials
        .clone()
        .or_else(|| connection.default_credentials());

    let stream_option = StreamOption::StreamIdentifier(stream_identifier);
    let stream_option = Some(stream_option);
    let options = Options {
        group_name: group_name.as_ref().to_string(),
        buffer_size: options.batch_size as i32,
        uuid_option: Some(uuid_option),
        stream_option,
    };

    let read_req = ReadReq {
        content: Some(read_req::Content::Options(options)),
    };

    let mut req = Request::new(recv);

    configure_auth_req(&mut req, credentials);

    let _ = sender.send(read_req).await;

    connection
        .execute(|channel| async {
            let mut client = PersistentSubscriptionsClient::new(channel.channel.clone());
            let mut stream = client.read(req).await?.into_inner();

            let stream = stream! {
                loop {
                    match stream.try_next().await {
                        Err(e) => {
                            let e = crate::Error::from_grpc(e);

                            channel.report_error(e.clone()).await;
                            yield Err(e);
                            break;
                        }

                        Ok(resp) => {
                            if let Some(resp) = resp {
                                match resp.content.expect("content is defined") {
                                    read_resp::Content::Event(event) => {
                                        yield Ok(SubEvent::EventAppeared(convert_persistent_proto_read_event(event)));
                                    }

                                    read_resp::Content::SubscriptionConfirmation(sub) => {
                                        yield Ok(SubEvent::Confirmed(sub.subscription_id));
                                    }
                                }
                                continue;
                            }

                            break;
                        }
                    }
                }
            };

            let read = SubscriptionRead {
                inner: Box::pin(stream),
            };
            let write = SubscriptionWrite { sender };

            Ok((read, write))
        })
        .await
}

pub struct SubscriptionRead {
    inner: BoxStream<'static, crate::Result<SubEvent>>,
}

impl SubscriptionRead {
    pub async fn try_next(&mut self) -> crate::Result<Option<SubEvent>> {
        self.inner.try_next().await
    }

    pub async fn try_next_event(&mut self) -> crate::Result<Option<ResolvedEvent>> {
        let event = self.inner.try_next().await?;

        if let Some(SubEvent::EventAppeared(event)) = event {
            return Ok(Some(event));
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
