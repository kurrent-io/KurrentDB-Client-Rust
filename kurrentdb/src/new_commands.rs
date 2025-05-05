use crate::{
    AppendRequest, AppendToStreamOptions, MultiWriteResult,
    commands::new_request,
    event_store::{
        client::MAX_RECEIVE_MESSAGE_SIZE,
        generated::new_streams::{
            MultiStreamAppendRequest, streams_service_client::StreamsServiceClient,
        },
    },
    grpc::{GrpcClient, handle_error},
};

/// Sends asynchronously the write command to the server.
pub async fn multi_stream_append(
    connection: &GrpcClient,
    options: &AppendToStreamOptions,
    mut events: impl Iterator<Item = AppendRequest> + Send + 'static,
) -> crate::Result<MultiWriteResult> {
    let (min, max) = events.size_hint();
    let sized = max == Some(min);

    if min == 0 && sized {
        return Err(crate::Error::IllegalOperation(
            "iterator won't yield a value".to_string(),
        ));
    }

    let handle = connection.current_selected_node().await?;
    let handle_id = handle.id();
    let mut client = StreamsServiceClient::with_interceptor(handle.client, handle.uri)
        .max_decoding_message_size(MAX_RECEIVE_MESSAGE_SIZE);

    let res = if min == 1 && sized {
        let req = events.next().expect("not be empty");
        let req = new_request(
            connection.connection_settings(),
            options,
            MultiStreamAppendRequest {
                input: vec![req.into()],
            },
        );

        client.multi_stream_append(req).await
    } else {
        // streaming
        let payload = async_stream::stream! {
            for req in events {
                yield req.into()
            }
        };

        let req = new_request(connection.connection_settings(), options, payload);
        client.multi_stream_append_session(req).await
    };

    // handle_error(&connection.sender, handle_id, &e);
    let resp = res
        .map_err(crate::Error::from_grpc)
        .inspect_err(|e| handle_error(&connection.sender, handle_id, e))?
        .into_inner();

    Ok(resp.into())
}
