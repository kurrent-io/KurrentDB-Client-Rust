#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;

mod images;

use eventstore::{
    Acl, Client, ClientSettings, EventData, PersistentSubscriptionOptions,
    PersistentSubscriptionToAllOptions, ProjectionClient, StreamAclBuilder, StreamMetadataBuilder,
    StreamMetadataResult,
};
use futures::channel::oneshot;
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;
use testcontainers::clients::Cli;
use testcontainers::{Docker, RunArgs};

fn fresh_stream_id(prefix: &str) -> String {
    let uuid = uuid::Uuid::new_v4();

    format!("{}-{}", prefix, uuid)
}

fn generate_events<Type: AsRef<str>>(event_type: Type, cnt: usize) -> Vec<EventData> {
    let mut events = Vec::with_capacity(cnt);

    for idx in 1..cnt + 1 {
        let payload = json!({
            "event_index": idx,
        });

        let data = EventData::json(event_type.as_ref(), payload).unwrap();
        events.push(data);
    }

    events
}

async fn test_write_events(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("write_events");
    let events = generate_events("es6-write-events-test".to_string(), 3);

    let result = client
        .append_to_stream(stream_id, &Default::default(), events)
        .await?;

    debug!("Write response: {:?}", result);
    assert_eq!(result.next_expected_version, 2);

    Ok(())
}

// We read all stream events by batch.
async fn test_read_all_stream_events(client: &Client) -> Result<(), Box<dyn Error>> {
    // Eventstore should always have "some" events in $all, since eventstore itself uses streams, ouroboros style.
    let result = client.read_all(&Default::default()).await?.next().await?;

    assert!(result.is_some());

    Ok(())
}

// We read stream events by batch. We also test if we can properly read a
// stream thoroughly.
async fn test_read_stream_events(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("read_stream_events");
    let events = generate_events("es6-read-stream-events-test".to_string(), 10);

    let _ = client
        .append_to_stream(stream_id.clone(), &Default::default(), events)
        .await?;

    let mut pos = 0usize;
    let mut idx = 0i64;

    let mut stream = client.read_stream(stream_id, &Default::default()).await?;

    while let Some(event) = stream.next().await? {
        let event = event.get_original_event();
        let obj: HashMap<String, i64> = event.as_json().unwrap();
        let value = obj.get("event_index").unwrap();

        idx = *value;
        pos += 1;
    }

    assert_eq!(pos, 10);
    assert_eq!(idx, 10);

    Ok(())
}

async fn test_metadata(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("metadata");
    let events = generate_events("metadata-test".to_string(), 5);

    let _ = client
        .append_to_stream(stream_id.as_str(), &Default::default(), events)
        .await?;

    let expected = StreamMetadataBuilder::new()
        .max_age(std::time::Duration::from_secs(2))
        .acl(Acl::Stream(
            StreamAclBuilder::new().add_read_roles("admin").build(),
        ))
        .build();

    let _ = client
        .set_stream_metadata(stream_id.as_str(), &Default::default(), expected.clone())
        .await?;

    let actual = client
        .get_stream_metadata(stream_id.as_str(), &Default::default())
        .await?;

    assert!(actual.is_success());

    if let StreamMetadataResult::Success(actual) = actual {
        assert_eq!(&expected, actual.metadata());
    }

    Ok(())
}

async fn test_metadata_not_exist(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("metadata_not_exist");
    let events = generate_events("metadata-test-not-exist".to_string(), 5);

    let _ = client
        .append_to_stream(stream_id.as_str(), &Default::default(), events)
        .await?;

    let actual = client
        .get_stream_metadata(stream_id.as_str(), &Default::default())
        .await?;

    assert!(actual.is_not_found());

    Ok(())
}

// We check to see the client can handle the correct GRPC proto response when
// a stream does not exist
async fn test_read_stream_events_non_existent(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("read_stream_events");

    let mut stream = client
        .read_stream(stream_id.as_str(), &Default::default())
        .await?;

    if let Err(eventstore::Error::ResourceNotFound) = stream.next().await {
        return Ok(());
    }

    panic!("We expected to have a stream not found result");
}

// We write an event into a stream then soft delete that stream.
async fn test_delete_stream(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("delete");
    let events = generate_events("delete-test".to_string(), 1);

    let _ = client
        .append_to_stream(stream_id.clone(), &Default::default(), events)
        .await?;

    let result = client
        .delete_stream(stream_id.as_str(), &Default::default())
        .await?;

    debug!("Delete stream [{}] result: {:?}", stream_id, result);

    Ok(())
}

// We write an event into a stream then hard delete that stream.
async fn test_tombstone_stream(client: &Client) -> Result<(), Box<dyn Error>> {
    let _ = pretty_env_logger::try_init();
    let stream_id = fresh_stream_id("tombstone");
    let events = generate_events("tombstone-test".to_string(), 1);

    let _ = client
        .append_to_stream(stream_id.clone(), &Default::default(), events)
        .await?;

    let result = client
        .tombstone_stream(stream_id.as_str(), &Default::default())
        .await?;

    debug!("Tombstone stream [{}] result: {:?}", stream_id, result);

    let result = client
        .read_stream(stream_id.as_str(), &Default::default())
        .await;

    if let Err(eventstore::Error::ResourceDeleted) = result {
        Ok(())
    } else {
        panic!("Expected stream deleted error");
    }
}

// We write events into a stream. Then, we issue a catchup subscription. After,
// we write another batch of events into the same stream. The goal is to make
// sure we receive events written prior and after our subscription request.
// To assess we received all the events we expected, we test our subscription
// internal state value.
async fn test_subscription(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("catchup");
    let events_before = generate_events("catchup-test-before".to_string(), 3);
    let events_after = generate_events("catchup-test-after".to_string(), 3);

    let _ = client
        .append_to_stream(stream_id.as_str(), &Default::default(), events_before)
        .await?;

    let options = eventstore::SubscribeToStreamOptions::default()
        .start_from(eventstore::StreamPosition::Start);

    let mut sub = client
        .subscribe_to_stream(stream_id.as_str(), &options)
        .await;

    let (tx, recv) = oneshot::channel();

    tokio::spawn(async move {
        let mut count = 0usize;
        let max = 6usize;

        loop {
            sub.next().await?;
            count += 1;

            if count == max {
                break;
            }
        }

        tx.send(count).unwrap();
        Ok(()) as eventstore::Result<()>
    });

    let _ = client
        .append_to_stream(stream_id, &Default::default(), events_after)
        .await?;

    match tokio::time::timeout(Duration::from_secs(60), recv).await {
        Ok(test_count) => {
            assert_eq!(
                test_count?, 6,
                "We are testing proper state after catchup subscription: got {} expected {}.",
                test_count?, 6
            );
        }

        Err(_) => panic!("test_subscription timed out!"),
    }

    Ok(())
}

async fn test_create_persistent_subscription(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("create_persistent_sub");

    client
        .create_persistent_subscription(stream_id, "a_group_name", &Default::default())
        .await?;

    Ok(())
}

async fn test_create_persistent_subscription_to_all(
    client: &Client,
    names: &mut names::Generator<'_>,
) -> eventstore::Result<()> {
    let group_name = names.next().unwrap();

    client
        .create_persistent_subscription_to_all(group_name, &Default::default())
        .await?;

    Ok(())
}

// We test we can successfully update a persistent subscription.
async fn test_update_persistent_subscription(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("update_persistent_sub");

    let mut options = PersistentSubscriptionOptions::default();

    client
        .create_persistent_subscription(stream_id.as_str(), "a_group_name", &options)
        .await?;

    options.settings_mut().max_retry_count = 1_000;

    client
        .update_persistent_subscription(stream_id, "a_group_name", &options)
        .await?;

    Ok(())
}

async fn test_update_persistent_subscription_to_all(
    client: &Client,
    names: &mut names::Generator<'_>,
) -> eventstore::Result<()> {
    let group_name = names.next().unwrap();

    let mut options = PersistentSubscriptionToAllOptions::default();

    client
        .create_persistent_subscription_to_all(group_name.as_str(), &options)
        .await?;

    options.settings_mut().max_retry_count = 1_000;

    client
        .update_persistent_subscription_to_all(group_name, &options)
        .await?;

    Ok(())
}

// We test we can successfully delete a persistent subscription.
async fn test_delete_persistent_subscription(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("delete_persistent_sub");

    client
        .create_persistent_subscription(stream_id.as_str(), "a_group_name", &Default::default())
        .await?;

    client
        .delete_persistent_subscription(stream_id, "a_group_name", &Default::default())
        .await?;

    Ok(())
}

async fn test_delete_persistent_subscription_to_all(
    client: &Client,
    names: &mut names::Generator<'_>,
) -> eventstore::Result<()> {
    let group_name = names.next().unwrap();

    client
        .create_persistent_subscription_to_all(group_name.as_str(), &Default::default())
        .await?;

    client
        .delete_persistent_subscription_to_all(group_name.as_str(), &Default::default())
        .await?;

    Ok(())
}

async fn test_persistent_subscription(client: &Client) -> eventstore::Result<()> {
    let stream_id = fresh_stream_id("persistent_subscription");
    let events = generate_events("es6-persistent-subscription-test".to_string(), 5);

    client
        .create_persistent_subscription(stream_id.as_str(), "a_group_name", &Default::default())
        .await?;

    let _ = client
        .append_to_stream(stream_id.as_str(), &Default::default(), events)
        .await?;

    let mut sub = client
        .subscribe_to_persistent_subscription(
            stream_id.as_str(),
            "a_group_name",
            &Default::default(),
        )
        .await?;

    let max = 10usize;

    let handle = tokio::spawn(async move {
        let mut count = 0usize;
        loop {
            let event = sub.next().await?;
            sub.ack(event).await?;

            count += 1;

            if count == max {
                break;
            }
        }

        Ok::<usize, eventstore::Error>(count)
    });

    let events = generate_events("es6-persistent-subscription-test".to_string(), 5);
    let _ = client
        .append_to_stream(stream_id.as_str(), &Default::default(), events)
        .await?;

    let count = handle
        .await
        .map_err(|_| {
            eventstore::Error::IllegalStateError(
                "Error when joining the tokio thread handle".to_string(),
            )
        })
        .and_then(|r| r)?;

    assert_eq!(
        count, 10,
        "We are testing proper state after persistent subscription: got {} expected {}",
        count, 10
    );

    Ok(())
}

async fn test_persistent_subscription_to_all(
    client: &Client,
    names: &mut names::Generator<'_>,
) -> eventstore::Result<()> {
    let group_name = names.next().unwrap();

    let options = eventstore::PersistentSubscriptionToAllOptions::default()
        .start_from(eventstore::StreamPosition::Start);

    client
        .create_persistent_subscription_to_all(group_name.as_str(), &options)
        .await?;

    let mut sub = client
        .subscribe_to_persistent_subscription_to_all(group_name, &Default::default())
        .await?;

    let limit = 3;
    let outcome = tokio::time::timeout(Duration::from_secs(60), async move {
        let mut count = 0;
        for _ in 0..limit + 1 {
            let event = sub.next().await?;
            sub.ack(event).await?;

            count += 1;

            if count == limit {
                break;
            }
        }

        Ok(count)
    })
    .await;

    match outcome {
        Ok(count) => {
            assert_eq!(count?, limit)
        }
        Err(_) => panic!("persistent subscription to $all test timed out!"),
    };

    Ok(())
}

async fn test_list_persistent_subscriptions(
    client: &Client,
    names: &mut names::Generator<'_>,
) -> eventstore::Result<()> {
    let mut count = 0;
    let group_name = "group_name";
    let mut expected_set = std::collections::HashSet::new();

    while let Some(stream_name) = names.next() {
        count += 1;
        client
            .create_persistent_subscription(stream_name.as_str(), &group_name, &Default::default())
            .await?;

        expected_set.insert(stream_name);
        if count > 2 {
            break;
        }
    }

    let mut actual_set = std::collections::HashSet::new();

    let ps = client
        .list_all_persistent_subscriptions(&Default::default())
        .await?;

    for info in ps {
        if expected_set.contains(info.event_stream_id.as_str()) {
            actual_set.insert(info.event_stream_id);
        }
    }

    assert_eq!(expected_set, actual_set);

    Ok(())
}

async fn test_list_persistent_subscriptions_for_stream(
    client: &Client,
    names: &mut names::Generator<'_>,
) -> eventstore::Result<()> {
    let mut count = 0;
    let stream_name = names.next().unwrap();
    let mut expected_set = std::collections::HashSet::new();

    while let Some(group_name) = names.next() {
        count += 1;
        client
            .create_persistent_subscription(
                stream_name.as_str(),
                group_name.as_str(),
                &Default::default(),
            )
            .await?;

        expected_set.insert(group_name);
        if count > 2 {
            break;
        }
    }

    let mut actual_set = std::collections::HashSet::new();

    let ps = client
        .list_persistent_subscriptions_for_stream(stream_name.as_str(), &Default::default())
        .await?;

    for info in ps {
        if info.event_stream_id != stream_name {
            continue;
        }

        if expected_set.contains(info.group_name.as_str()) {
            actual_set.insert(info.group_name);
        }
    }

    assert_eq!(expected_set, actual_set);

    Ok(())
}

async fn test_list_persistent_subscriptions_to_all(
    client: &Client,
    names: &mut names::Generator<'_>,
) -> eventstore::Result<()> {
    let mut count = 0;
    let mut expected_set = std::collections::HashSet::new();

    while let Some(group_name) = names.next() {
        count += 1;
        client
            .create_persistent_subscription_to_all(group_name.as_str(), &Default::default())
            .await?;

        expected_set.insert(group_name);
        if count > 2 {
            break;
        }
    }

    let mut actual_set = std::collections::HashSet::new();

    let ps = client
        .list_persistent_subscriptions_to_all(&Default::default())
        .await?;

    for info in ps {
        if expected_set.contains(info.group_name.as_str()) {
            actual_set.insert(info.group_name);
        }
    }

    assert_eq!(expected_set, actual_set);

    Ok(())
}

async fn test_get_persistent_subscription_info(
    client: &Client,
    names: &mut names::Generator<'_>,
) -> eventstore::Result<()> {
    let stream_name = names.next().unwrap();
    let group_name = names.next().unwrap();

    client
        .create_persistent_subscription(
            stream_name.as_str(),
            group_name.as_str(),
            &Default::default(),
        )
        .await?;

    let info = client
        .get_persistent_subscription_info(
            stream_name.as_str(),
            group_name.as_str(),
            &Default::default(),
        )
        .await?;

    assert_eq!(info.event_stream_id, stream_name);
    assert_eq!(info.group_name, group_name);
    assert!(info.config.is_some());

    Ok(())
}

async fn test_get_persistent_subscription_info_to_all(
    client: &Client,
    names: &mut names::Generator<'_>,
) -> eventstore::Result<()> {
    let group_name = names.next().unwrap();

    client
        .create_persistent_subscription_to_all(group_name.as_str(), &Default::default())
        .await?;

    let info = client
        .get_persistent_subscription_info_to_all(group_name.as_str(), &Default::default())
        .await?;

    assert_eq!(info.group_name, group_name);
    assert!(info.config.is_some());

    Ok(())
}

async fn test_replay_parked_messages(
    client: &Client,
    names: &mut names::Generator<'_>,
) -> eventstore::Result<()> {
    let stream_name = names.next().unwrap();
    let group_name = names.next().unwrap();
    let event_count = 2;

    client
        .create_persistent_subscription(
            stream_name.as_str(),
            group_name.as_str(),
            &Default::default(),
        )
        .await?;

    let mut sub = client
        .subscribe_to_persistent_subscription(
            stream_name.as_str(),
            group_name.as_str(),
            &Default::default(),
        )
        .await?;

    let events = generate_events("foobar", 2);

    client
        .append_to_stream(stream_name.as_str(), &Default::default(), events)
        .await?;

    let outcome = tokio::time::timeout(Duration::from_secs(30), async move {
        for _ in 0..event_count {
            let event = sub.next().await?;

            sub.nack(event, eventstore::NakAction::Park, "because reasons")
                .await?;
        }

        debug!("We let the server the time to write those parked event in the stream...");
        tokio::time::sleep(Duration::from_secs(5)).await;
        debug!("done.");

        debug!("Before replaying parked messages...");
        client
            .replay_parked_messages(stream_name, group_name, &Default::default())
            .await?;
        debug!("done.");

        for _ in 0..event_count {
            debug!("Waiting on parked event to be replayed...");
            let event = sub.next().await?;
            debug!("done.");
            sub.ack(event).await?;
        }

        Ok::<(), eventstore::Error>(())
    })
    .await;

    match outcome {
        Err(_) => panic!("test_replay_parked_messages timed out!"),
        Ok(outcome) => {
            assert!(outcome.is_ok());

            Ok(())
        }
    }
}

async fn test_replay_parked_messages_to_all(
    client: &Client,
    names: &mut names::Generator<'_>,
) -> eventstore::Result<()> {
    let stream_name = names.next().unwrap();
    let group_name = names.next().unwrap();
    let event_count = 2;

    client
        .create_persistent_subscription_to_all(group_name.as_str(), &Default::default())
        .await?;

    let mut sub = client
        .subscribe_to_persistent_subscription_to_all(group_name.as_str(), &Default::default())
        .await?;

    let events = generate_events("foobar", 2);

    client
        .append_to_stream(stream_name.as_str(), &Default::default(), events)
        .await?;

    let outcome = tokio::time::timeout(Duration::from_secs(30), async move {
        let mut count = 0;
        loop {
            let event = sub.next().await?;

            if event.get_original_stream_id() == stream_name.as_str() {
                sub.nack(event, eventstore::NakAction::Park, "because reasons")
                    .await?;

                count += 1;
                if count == event_count {
                    break;
                }
            } else {
                sub.ack(event).await?;
            }
        }

        debug!("We let the server the time to write those parked event in the stream...");
        tokio::time::sleep(Duration::from_secs(5)).await;
        debug!("done.");

        debug!("Before replaying parked messages...");
        client
            .replay_parked_messages_to_all(group_name, &Default::default())
            .await?;
        debug!("done.");

        count = 0;
        loop {
            debug!("Waiting on parked event to be replayed...");
            let event = sub.next().await?;
            debug!("done.");

            let event_stream_id = event.event.as_ref().unwrap().stream_id.to_string();
            sub.ack(event).await?;

            if event_stream_id == stream_name.as_str() {
                count += 1;
                if count == event_count {
                    break;
                }
            }
        }

        Ok::<(), eventstore::Error>(())
    })
    .await;

    match outcome {
        Err(_) => panic!("test_replay_parked_messages_to_all timed out!"),
        Ok(outcome) => {
            assert!(outcome.is_ok());

            Ok(())
        }
    }
}

async fn test_restart_persistent_subscription_subsystem(client: &Client) -> eventstore::Result<()> {
    client
        .restart_persistent_subscription_subsystem(&Default::default())
        .await
}

async fn test_persistent_subscription_encoding(
    client: &Client,
    names: &mut names::Generator<'_>,
) -> eventstore::Result<()> {
    let stream_name = format!("/{}/foo", names.next().unwrap());
    let group_name = format!("/{}/foo", names.next().unwrap());

    client
        .create_persistent_subscription(
            stream_name.as_str(),
            group_name.as_str(),
            &Default::default(),
        )
        .await?;

    let info = client
        .get_persistent_subscription_info(
            stream_name.as_str(),
            group_name.as_str(),
            &Default::default(),
        )
        .await?;

    assert_eq!(info.event_stream_id, stream_name);
    assert_eq!(info.group_name, group_name);

    Ok(())
}

async fn test_batch_append(client: &Client) -> eventstore::Result<()> {
    let batch_client = client.batch_append(&Default::default()).await?;

    for _ in 0..3 {
        let stream_id = fresh_stream_id("batch-append");
        let events = generate_events("batch-append-type", 3);
        let _ = batch_client
            .append_to_stream(
                stream_id.as_str(),
                eventstore::ExpectedRevision::Any,
                events,
            )
            .await?;
        let options = eventstore::ReadStreamOptions::default()
            .forwards()
            .position(eventstore::StreamPosition::Start);
        let mut stream = client.read_stream(stream_id.as_str(), &options).await?;

        let mut cpt = 0usize;

        while let Some(_) = stream.next().await? {
            cpt += 1;
        }

        assert_eq!(cpt, 3, "We expecting 3 events out of those streams");
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_error_on_failure_to_discover_single_node() -> Result<(), Box<dyn Error>> {
    let _ = pretty_env_logger::try_init();

    let settings = format!("esdb://noserver:{}", 2_113).parse()?;
    let client = Client::new(settings)?;
    let stream_id = fresh_stream_id("wont-be-created");
    let events = generate_events("wont-be-written".to_string(), 5);

    let result = client
        .append_to_stream(stream_id, &Default::default(), events)
        .await;

    if let Err(eventstore::Error::GrpcConnectionError(_)) = result {
        Ok(())
    } else {
        panic!("Expected gRPC connection error");
    }
}

type VolumeName = String;

fn create_unique_volume() -> Result<VolumeName, Box<dyn std::error::Error>> {
    let dir_name = uuid::Uuid::new_v4();
    let dir_name = format!("dir-{}", dir_name);

    std::process::Command::new("docker")
        .arg("volume")
        .arg("create")
        .arg(format!("--name {}", dir_name))
        .output()?;

    Ok(dir_name)
}

async fn wait_node_is_alive(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    match tokio::time::timeout(std::time::Duration::from_secs(60), async move {
        loop {
            match tokio::time::timeout(
                std::time::Duration::from_secs(1),
                reqwest::get(format!("http://localhost:{}/health/live", port)),
            )
            .await
            {
                Err(_) => error!("Healthcheck timed out! retrying..."),

                Ok(resp) => match resp {
                    Err(e) => error!("Node localhost:{} is not up yet: {}", port, e),

                    Ok(resp) => {
                        if resp.status().is_success() {
                            break;
                        }

                        error!(
                            "Healthcheck response was not successful: {}, retrying...",
                            resp.status()
                        );
                    }
                },
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    })
    .await
    {
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "Docker container took too much time to start",
        )
        .into()),
        Ok(_) => {
            debug!(
                "Docker container was started successfully on localhost:{}",
                port
            );

            Ok(())
        }
    }
}

// This function assumes that we are using the admin credentials. It's possible during CI that
// the cluster hasn't created the admin user yet, leading to failing the tests.
async fn wait_for_admin_to_be_available(client: &eventstore::Client) -> eventstore::Result<()> {
    fn can_retry(e: &eventstore::Error) -> bool {
        match e {
            eventstore::Error::AccessDenied
            | eventstore::Error::DeadlineExceeded
            | eventstore::Error::ServerError(_)
            | eventstore::Error::NotLeaderException(_)
            | eventstore::Error::ResourceNotFound => true,
            _ => false,
        }
    }
    let mut count = 0;

    while count < 50 {
        count += 1;

        debug!("Checking if admin user is available...{}/50", count);
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), async move {
            let mut stream = client.read_stream("$users", &Default::default()).await?;
            stream.next().await
        })
        .await;

        match result {
            Err(_) => {
                debug!("Request timed out, retrying...");
                tokio::time::sleep(Duration::from_millis(500)).await;
            }

            Ok(result) => match result {
                Err(e) if can_retry(&e) => {
                    debug!("Not available: {:?}, retrying...", e);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }

                Err(e) => {
                    debug!("Fatal error, stop retrying. Cause: {:?}", e);
                    return Err(e);
                }

                Ok(opt) => {
                    if opt.is_some() {
                        debug!("Admin account is available!");
                        return Ok(());
                    }

                    debug!("$users stream seems to be empty, retrying...");
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            },
        }
    }

    Err(eventstore::Error::ServerError(
        "Waiting for the admin user to be created took too much time".to_string(),
    ))
}

#[tokio::test(flavor = "multi_thread")]
async fn cluster() -> Result<(), Box<dyn std::error::Error>> {
    let _ = pretty_env_logger::try_init();
    let settings = "esdb://admin:changeit@localhost:2111,localhost:2112,localhost:2113?tlsVerifyCert=false&nodePreference=leader&maxdiscoverattempts=50"
        .parse::<ClientSettings>()?;

    let client = Client::new(settings)?;

    // Those pre-checks are put in place to avoid test flakiness. In essence, those functions use
    // features we test later on.
    wait_for_admin_to_be_available(&client).await?;

    all_around_tests(client).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn single_node() -> Result<(), Box<dyn std::error::Error>> {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();
    let image = images::ESDB::default().insecure_mode();
    let container = docker.run_with_args(image, RunArgs::default());

    wait_node_is_alive(container.get_host_port(2_113).unwrap()).await?;

    let settings = format!(
        "esdb://localhost:{}?tls=false",
        container.get_host_port(2_113).unwrap(),
    )
    .parse::<ClientSettings>()?;

    let client = Client::new(settings)?;

    all_around_tests(client).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_auto_resub_on_connection_drop() -> Result<(), Box<dyn std::error::Error>> {
    let volume = create_unique_volume()?;
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();
    let image = images::ESDB::default()
        .insecure_mode()
        .attach_volume_to_db_directory(volume);
    let container = docker.run_with_args(
        image.clone(),
        RunArgs::default().with_mapped_port((3_113, 2_113)),
    );

    wait_node_is_alive(3_113).await?;

    let settings = format!("esdb://localhost:{}?tls=false", 3_113).parse::<ClientSettings>()?;

    let client = Client::new(settings)?;
    let stream_name = fresh_stream_id("auto-reconnect");
    let retry = eventstore::RetryOptions::default().retry_forever();
    let options = eventstore::SubscribeToStreamOptions::default().retry_options(retry);
    let mut stream = client
        .subscribe_to_stream(stream_name.as_str(), &options)
        .await;
    let max = 6usize;
    let (tx, recv) = oneshot::channel();

    tokio::spawn(async move {
        let mut count = 0usize;

        loop {
            stream.next().await?;
            count += 1;

            if count == max {
                break;
            }
        }

        tx.send(count).unwrap();
        Ok(()) as eventstore::Result<()>
    });

    let events = generate_events("reconnect".to_string(), 3);

    let _ = client
        .append_to_stream(stream_name.as_str(), &Default::default(), events)
        .await?;

    container.stop();
    let _container = docker.run_with_args(
        image.clone(),
        RunArgs::default().with_mapped_port((3_113, 2_113)),
    );

    wait_node_is_alive(3_113).await?;

    let events = generate_events("reconnect".to_string(), 3);

    let _ = client
        .append_to_stream(stream_name.as_str(), &Default::default(), events)
        .await?;
    let test_count = recv.await?;

    assert_eq!(
        test_count, 6,
        "We are testing proper state after subscription upon reconnection: got {} expected {}.",
        test_count, 6
    );

    Ok(())
}

async fn all_around_tests(client: Client) -> Result<(), Box<dyn std::error::Error>> {
    let mut name_generator = names::Generator::default();

    debug!("Before test_write_events…");
    test_write_events(&client).await?;
    debug!("Complete");
    debug!("Before test_all_read_stream_events…");
    test_read_all_stream_events(&client).await?;
    debug!("Complete");
    debug!("Before test_read_stream_events…");
    test_read_stream_events(&client).await?;
    debug!("Complete");
    debug!("Before test_read_stream_events_non_existent");
    test_read_stream_events_non_existent(&client).await?;
    debug!("Complete");
    debug!("Before test test_metadata");
    test_metadata(&client).await?;
    debug!("Complete");
    debug!("Before test test_metadata_not_exist");
    test_metadata_not_exist(&client).await?;
    debug!("Complete");
    debug!("Before test_delete_stream…");
    test_delete_stream(&client).await?;
    debug!("Complete");
    debug!("Before test_tombstone_stream…");
    test_tombstone_stream(&client).await?;
    debug!("Complete");
    debug!("Before test_subscription…");
    test_subscription(&client).await?;
    debug!("Complete");
    debug!("Before test_create_persistent_subscription…");
    test_create_persistent_subscription(&client).await?;
    debug!("Complete");
    debug!("Before test_update_persistent_subscription…");
    test_update_persistent_subscription(&client).await?;
    debug!("Complete");
    debug!("Before test_create_persistent_subscription_to_all");
    if let Err(e) = test_create_persistent_subscription_to_all(&client, &mut name_generator).await {
        if let eventstore::Error::UnsupportedFeature = e {
            warn!(
                "Persistent subscription to $all is not supported on the server we are targeting"
            );
            Ok(())
        } else {
            Err(e)
        }?;
    }
    debug!("Complete");
    debug!("Before test_update_persistent_subscription_to_all");
    if let Err(e) = test_update_persistent_subscription_to_all(&client, &mut name_generator).await {
        if let eventstore::Error::UnsupportedFeature = e {
            warn!(
                "Persistent subscription to $all is not supported on the server we are targeting"
            );
            Ok(())
        } else {
            Err(e)
        }?;
    }
    debug!("Complete");
    debug!("Before test_delete_persistent_subscription…");
    test_delete_persistent_subscription(&client).await?;
    debug!("Complete");
    debug!("Before test_delete_persistent_subscription_to_all");
    if let Err(e) = test_delete_persistent_subscription_to_all(&client, &mut name_generator).await {
        if let eventstore::Error::UnsupportedFeature = e {
            warn!(
                "Persistent subscription to $all is not supported on the server we are targeting"
            );
            Ok(())
        } else {
            Err(e)
        }?;
    }
    debug!("Complete");
    debug!("Before test_persistent_subscription…");
    test_persistent_subscription(&client).await?;
    debug!("Complete");
    debug!("Before test_persistent_subscription_to_all");
    if let Err(e) = test_persistent_subscription_to_all(&client, &mut name_generator).await {
        if let eventstore::Error::UnsupportedFeature = e {
            warn!(
                "Persistent subscription to $all is not supported on the server we are targeting"
            );
            Ok(())
        } else {
            Err(e)
        }?;
    }
    debug!("Complete");
    debug!("Before test_list_persistent_subscriptions...");
    test_list_persistent_subscriptions(&client, &mut name_generator).await?;
    debug!("Complete");
    debug!("Before test_list_persistent_subscriptions_for_stream...");
    test_list_persistent_subscriptions_for_stream(&client, &mut name_generator).await?;
    debug!("Complete");
    debug!("Before test_list_persistent_subscriptions_to_all...");
    if let Err(e) = test_list_persistent_subscriptions_to_all(&client, &mut name_generator).await {
        if let eventstore::Error::UnsupportedFeature = e {
            warn!(
                "Persistent subscription to $all is not supported on the server we are targeting"
            );
            Ok(())
        } else {
            Err(e)
        }?;
    }
    debug!("Complete");
    debug!("Before test_get_persistent_subscription_info...");
    test_get_persistent_subscription_info(&client, &mut name_generator).await?;
    debug!("Complete");
    debug!("Before test_get_persistent_subscription_info_to_all...");
    if let Err(e) = test_get_persistent_subscription_info_to_all(&client, &mut name_generator).await
    {
        if let eventstore::Error::UnsupportedFeature = e {
            warn!(
                "Persistent subscription to $all is not supported on the server we are targeting"
            );
            Ok(())
        } else {
            Err(e)
        }?;
    }
    debug!("Complete");
    debug!("Before test_replay_parked_messages...");
    test_replay_parked_messages(&client, &mut name_generator).await?;
    debug!("Complete");
    debug!("Before test_replay_parked_messages_to_all...");
    if let Err(e) = test_replay_parked_messages_to_all(&client, &mut name_generator).await {
        if let eventstore::Error::UnsupportedFeature = e {
            warn!(
                "Persistent subscription to $all is not supported on the server we are targeting"
            );
            Ok(())
        } else {
            Err(e)
        }?;
    }
    debug!("Complete");
    debug!("Before test_restart_persistent_subscription_subsystem...");
    test_restart_persistent_subscription_subsystem(&client).await?;
    debug!("Complete");
    debug!("Before test_persistent_subscription_encoding...");
    test_persistent_subscription_encoding(&client, &mut name_generator).await?;
    debug!("Complete");
    debug!("Before test_batch_append");
    if let Err(e) = test_batch_append(&client).await {
        if let eventstore::Error::UnsupportedFeature = e {
            warn!("batch_append is not supported on the server we are targeting");
            Ok(())
        } else {
            Err(e)
        }?;
    }
    debug!("Complete");

    Ok(())
}

static PROJECTION_FILE: &'static str = include_str!("fixtures/projection.js");
static PROJECTION_UPDATED_FILE: &'static str = include_str!("fixtures/projection-updated.js");

async fn wait_until_projection_status_cc(
    client: &ProjectionClient,
    name: &str,
    last_status: &mut String,
    status: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let result = client.get_status(name, &Default::default()).await?;

        if let Some(stats) = result {
            if stats.status.contains(status) {
                break;
            }

            *last_status = stats.status.clone();
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    Ok(())
}

async fn wait_until_projection_status_is(
    client: &ProjectionClient,
    name: &str,
    status: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut last_status = "".to_string();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(FIVE_MINS_IN_SECS),
        wait_until_projection_status_cc(client, name, &mut last_status, status),
    )
    .await;

    if result.is_err() {
        error!(
            "Projection {} doesn't reach the expected status. Got {}, Expected {}",
            name, last_status, status
        );
    }

    result?
}

async fn wait_until_state_ready<A>(
    client: &ProjectionClient,
    name: &str,
) -> Result<A, Box<dyn std::error::Error>>
where
    A: serde::de::DeserializeOwned + Send,
{
    tokio::time::timeout(
        std::time::Duration::from_secs(FIVE_MINS_IN_SECS),
        async move {
            loop {
                let result: serde_json::Result<A> =
                    client.get_state(name, &Default::default()).await?;

                if let Ok(a) = result {
                    return Ok::<A, Box<dyn std::error::Error>>(a);
                }
            }
        },
    )
    .await?
}

async fn wait_until_result_ready<A>(
    client: &ProjectionClient,
    name: &str,
) -> Result<A, Box<dyn std::error::Error>>
where
    A: serde::de::DeserializeOwned + Send,
{
    tokio::time::timeout(
        std::time::Duration::from_secs(FIVE_MINS_IN_SECS),
        async move {
            loop {
                let result: serde_json::Result<A> =
                    client.get_result(name, &Default::default()).await?;

                if let Ok(a) = result {
                    return Ok::<A, Box<dyn std::error::Error>>(a);
                }
            }
        },
    )
    .await?
}

const FIVE_MINS_IN_SECS: u64 = 5 * 60;

async fn create_projection(
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = gen_name.next().unwrap();
    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await
}

// TODO - A projection must be stopped to be able to delete it. But Stop projection gRPC call doesn't exist yet.
async fn delete_projection(
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = gen_name.next().unwrap();
    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;

    debug!("delete_projection: create_projection succeeded: {}", name);

    client.abort(name.as_str(), &Default::default()).await?;

    wait_until_projection_status_is(client, name.as_str(), "Aborted").await?;

    debug!("delete_projection: reading newly-created projection statistic succeeded");

    let cloned_name = name.clone();
    // There is a race-condition in the projection manager: https://github.com/EventStore/EventStore/issues/2938
    let result = tokio::time::timeout(std::time::Duration::from_secs(10), async move {
        loop {
            let result = client
                .delete(cloned_name.as_str(), &Default::default())
                .await;

            if result.is_ok() {
                break;
            }

            warn!("projection deletion failed with: {:?}. Retrying...", result);
            let _ = tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    })
    .await;

    if result.is_err() {
        warn!("projection deletion didn't complete under test timeout. Not a big deal considering https://github.com/EventStore/EventStore/issues/2938");
    }

    Ok(())
}
async fn update_projection(
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = gen_name.next().unwrap();

    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;

    client
        .update(
            name.as_str(),
            PROJECTION_UPDATED_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    let stats = client
        .get_status(name.as_str(), &Default::default())
        .await?;

    assert!(stats.is_some());

    let stats = stats.unwrap();

    assert_eq!(stats.name, name);
    assert_eq!(stats.version, 1);

    Ok(())
}

async fn enable_projection(
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = gen_name.next().unwrap();
    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;

    client.enable(name.as_str(), &Default::default()).await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;

    Ok(())
}

async fn disable_projection(
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = gen_name.next().unwrap();
    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;
    client.enable(name.as_str(), &Default::default()).await?;
    wait_until_projection_status_is(client, name.as_str(), "Running").await?;
    client.disable(name.as_str(), &Default::default()).await?;
    wait_until_projection_status_is(client, name.as_str(), "Stopped").await?;

    Ok(())
}

async fn reset_projection(
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let name = gen_name.next().unwrap();
    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;
    client.enable(name.as_str(), &Default::default()).await?;
    client.reset(name.as_str(), &Default::default()).await?;

    Ok(())
}

async fn projection_state(
    stream_client: &Client,
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    use serde::Deserialize;

    let events = generate_events("testing", 10);
    let stream_name = gen_name.next().unwrap();

    stream_client
        .append_to_stream(stream_name, &Default::default(), events)
        .await?;

    // This is the state of the projection, see tests/fixtures/projection.js.
    #[derive(Deserialize, Debug)]
    struct State {
        foo: Foo,
    }

    #[derive(Deserialize, Debug)]
    struct Foo {
        baz: Baz,
    }

    #[derive(Deserialize, Debug)]
    struct Baz {
        count: f64,
    }

    let name = gen_name.next().unwrap();
    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;
    client.enable(name.as_str(), &Default::default()).await?;

    let state = wait_until_state_ready::<State>(client, name.as_str()).await?;

    debug!("{:?}", state);

    Ok(())
}

async fn projection_result(
    stream_client: &Client,
    client: &ProjectionClient,
    gen_name: &mut names::Generator<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    use serde::Deserialize;

    let events = generate_events("testing", 10);
    let stream_name = gen_name.next().unwrap();

    stream_client
        .append_to_stream(stream_name, &Default::default(), events)
        .await?;

    // This is the state of the projection, see tests/fixtures/projection.js.
    #[derive(Deserialize, Debug)]
    struct State {
        foo: Foo,
    }

    #[derive(Deserialize, Debug)]
    struct Foo {
        baz: Baz,
    }

    #[derive(Deserialize, Debug)]
    struct Baz {
        count: f64,
    }

    let name = gen_name.next().unwrap();
    client
        .create(
            name.as_str(),
            PROJECTION_FILE.to_string(),
            &Default::default(),
        )
        .await?;

    wait_until_projection_status_is(client, name.as_str(), "Running").await?;
    client.enable(name.as_str(), &Default::default()).await?;

    let result = wait_until_result_ready::<State>(client, name.as_str()).await?;

    debug!("{:?}", result);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn projection_tests() -> Result<(), Box<dyn std::error::Error>> {
    let _ = pretty_env_logger::try_init();
    let docker = Cli::default();
    let image = images::ESDB::default().insecure_mode().enable_projections();
    let container = docker.run_with_args(image, RunArgs::default());

    wait_node_is_alive(container.get_host_port(2_113).unwrap()).await?;

    let settings = format!(
        "esdb://localhost:{}?tls=false",
        container.get_host_port(2_113).unwrap(),
    )
    .parse::<ClientSettings>()?;

    let client = ProjectionClient::new(settings.clone());
    let stream_client = Client::new(settings)?;
    let mut name_gen = names::Generator::default();

    match tokio::time::timeout(std::time::Duration::from_secs(15 * 60), async move {
        create_projection(&client, &mut name_gen).await?;
        debug!("create_projection passed");
        delete_projection(&client, &mut name_gen).await?;
        debug!("delete_projection passed");
        update_projection(&client, &mut name_gen).await?;
        debug!("update_projection passed");
        enable_projection(&client, &mut name_gen).await?;
        debug!("enable_projection passed");
        disable_projection(&client, &mut name_gen).await?;
        debug!("disable_projection passed");
        reset_projection(&client, &mut name_gen).await?;
        debug!("reset_projection passed");
        projection_state(&stream_client, &client, &mut name_gen).await?;
        debug!("projection_state passed");
        projection_result(&stream_client, &client, &mut name_gen).await?;
        debug!("projection_result passed");

        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await
    {
        Err(_) => panic!("Running projection tests took too much time!"),
        Ok(outcome) => outcome,
    }
}
