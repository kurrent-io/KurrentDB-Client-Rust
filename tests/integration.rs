#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;

mod images;

use eventstore::{
    Batch, Client, ClientSettings, EventData, ExpectedRevision, PersistentSubscriptionOptions,
    PersistentSubscriptionSettings, Single,
};
use futures::channel::oneshot;
use futures::stream::TryStreamExt;
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::error::Error;
use std::pin::Pin;
use testcontainers::clients::Cli;
use testcontainers::{Docker, RunArgs};

fn fresh_stream_id(prefix: &str) -> String {
    let uuid = uuid::Uuid::new_v4();

    format!("{}-{}", prefix, uuid)
}

fn generate_events(event_type: String, cnt: usize) -> Vec<EventData> {
    let mut events = Vec::with_capacity(cnt);

    for idx in 1..cnt + 1 {
        let payload = json!({
            "event_index": idx,
        });

        let data = EventData::json(event_type.clone(), payload).unwrap();
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

    Ok(())
}

// We read all stream events by batch.
async fn test_read_all_stream_events(client: &Client) -> Result<(), Box<dyn Error>> {
    // Eventstore should always have "some" events in $all, since eventstore itself uses streams, ouroboros style.
    client.read_all(&Default::default(), Single).await?;

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

    let result = client
        .read_stream(stream_id, &Default::default(), 10)
        .await?;

    if let eventstore::ReadResult::Ok(mut stream) = result {
        while let Some(event) = stream.try_next().await? {
            let event = event.get_original_event();
            let obj: HashMap<String, i64> = event.as_json().unwrap();
            let value = obj.get("event_index").unwrap();

            idx = *value;
            pos += 1;
        }
    }

    assert_eq!(pos, 10);
    assert_eq!(idx, 10);

    Ok(())
}

// We check to see the client can handle the correct GRPC proto response when
// a stream does not exist
async fn test_read_stream_events_non_existent(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("read_stream_events");

    let result = client
        .read_stream(stream_id.as_str(), &Default::default(), Single)
        .await?;

    if let eventstore::ReadResult::StreamNotFound(stream) = result {
        assert_eq!(stream, stream_id);
        return Ok(());
    }

    panic!("We expected to have a stream not found result");
}

// We write an event into a stream then delete that stream.
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

    let mut sub = client
        .subscribe_to_stream(stream_id.as_str(), &Default::default())
        .await?;

    let (tx, recv) = oneshot::channel();

    tokio::spawn(async move {
        let mut count = 0usize;
        let max = 6usize;

        while let Some(event) = sub.try_next().await? {
            if let eventstore::SubEvent::EventAppeared(_) = event {
                count += 1;

                if count == max {
                    break;
                }
            }
        }

        tx.send(count).unwrap();
        Ok(()) as eventstore::Result<()>
    });

    let _ = client
        .append_to_stream(stream_id, &Default::default(), events_after)
        .await?;

    let test_count = recv.await?;

    assert_eq!(
        test_count, 6,
        "We are testing proper state after catchup subscription: got {} expected {}.",
        test_count, 6
    );

    Ok(())
}

async fn test_create_persistent_subscription(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("create_persistent_sub");

    client
        .create_persistent_subscription(stream_id, "a_group_name", &Default::default())
        .await?;

    Ok(())
}

// We test we can successfully update a persistent subscription.
async fn test_update_persistent_subscription(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("update_persistent_sub");

    client
        .create_persistent_subscription(stream_id.as_str(), "a_group_name", &Default::default())
        .await?;

    let mut setts = PersistentSubscriptionSettings::default();

    setts.max_retry_count = 1000;

    let options = PersistentSubscriptionOptions::default().settings(setts);
    client
        .update_persistent_subscription(stream_id, "a_group_name", &options)
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

async fn test_persistent_subscription(client: &Client) -> Result<(), Box<dyn Error>> {
    let stream_id = fresh_stream_id("persistent_subscription");
    let events = generate_events("es6-persistent-subscription-test".to_string(), 5);

    client
        .create_persistent_subscription(stream_id.as_str(), "a_group_name", &Default::default())
        .await?;

    let _ = client
        .append_to_stream(stream_id.as_str(), &Default::default(), events)
        .await?;

    let (mut read, mut write) = client
        .connect_persistent_subscription(stream_id.as_str(), "a_group_name", &Default::default())
        .await?;

    let max = 10usize;

    let handle = tokio::spawn(async move {
        let mut count = 0usize;
        while let Some(event) = read.try_next().await.unwrap() {
            if let eventstore::SubEvent::EventAppeared(event) = event {
                write.ack_event(event).await.unwrap();

                count += 1;

                if count == max {
                    break;
                }
            }
        }

        count
    });

    let events = generate_events("es6-persistent-subscription-test".to_string(), 5);
    let _ = client
        .append_to_stream(stream_id.as_str(), &Default::default(), events)
        .await?;

    let count = handle.await?;

    assert_eq!(
        count, 10,
        "We are testing proper state after persistent subscription: got {} expected {}",
        count, 10
    );

    Ok(())
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
    loop {
        let resp = reqwest::get(format!("http://localhost:{}/health/live", port)).await;

        if let Ok(resp) = resp {
            if resp.status().is_success() {
                break;
            }
        }

        warn!("Node localhost:{} is not up yet!", port);

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn cluster() -> Result<(), Box<dyn std::error::Error>> {
    let _ = pretty_env_logger::try_init();
    let settings = "esdb://admin:changeit@localhost:2111,localhost:2112,localhost:2113?tlsVerifyCert=false&nodePreference=leader"
        .parse::<ClientSettings>()?;

    let client = Client::create(settings).await?;

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
        container.get_host_port(2_113).unwrap()
    )
    .parse::<ClientSettings>()?;

    let client = Client::create(settings).await?;

    all_around_tests(client).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_batch_append() -> Result<(), Box<dyn std::error::Error>> {
    let _ = pretty_env_logger::try_init();

    let settings = format!("esdb://localhost:{}?tls=false", 2_113,).parse::<ClientSettings>()?;

    let client = Client::create(settings).await?;
    let mut batches = Vec::new();

    let events = generate_events("batch-foo".to_string(), 10);
    batches.push(Batch {
        correlation_id: uuid::Uuid::new_v4(),
        stream_name: "foo".to_string(),
        expected_version: ExpectedRevision::Any,
        events,
        is_final: false,
    });

    let events = generate_events("batch-bar".to_string(), 10);
    batches.push(Batch {
        correlation_id: uuid::Uuid::new_v4(),
        stream_name: "bar".to_string(),
        expected_version: ExpectedRevision::Any,
        events,
        is_final: false,
    });

    let events = generate_events("batch-baz".to_string(), 10);
    batches.push(Batch {
        correlation_id: uuid::Uuid::new_v4(),
        stream_name: "baz".to_string(),
        expected_version: ExpectedRevision::Any,
        events,
        is_final: true,
    });

    let mut output = client
        .batch_append(None, futures::stream::iter(batches))
        .await?;

    println!("Passed: {:?}", output.next().await);

    while let Some(resp) = output.try_next().await? {
        println!("batch resp: {:?}", resp);
    }

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

    let client = Client::create(settings).await?;
    let stream_name = fresh_stream_id("auto-reconnect");
    let retry = eventstore::RetryOptions::default().retry_forever();
    let options = eventstore::SubscribeToStreamOptions::default().retry_options(retry);
    let mut stream = client
        .subscribe_to_stream(stream_name.as_str(), &options)
        .await?;
    let max = 6usize;
    let (tx, recv) = oneshot::channel();

    tokio::spawn(async move {
        let mut count = 0usize;

        while let Some(_) = stream.try_next().await? {
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
    debug!("Before test_delete_stream…");
    test_delete_stream(&client).await?;
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
    debug!("Before test_delete_persistent_subscription…");
    test_delete_persistent_subscription(&client).await?;
    debug!("Complete");
    debug!("Before test_persistent_subscription…");
    test_persistent_subscription(&client).await?;
    debug!("Complete");

    Ok(())
}
