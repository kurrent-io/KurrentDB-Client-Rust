---
order: 3
---

# Reading Events

EventStoreDB provides two primary methods for reading events: reading from an
individual stream to retrieve events from a specific named stream, or reading
from the `$all` stream to access all events across the entire event store.

Events in EventStoreDB are organized within individual streams and use two
distinct positioning systems to track their location. The **revision number** is
a 64-bit signed integer (`long`) that represents the sequential position of an
event within its specific stream. Events are numbered starting from 0, with each
new event receiving the next sequential revision number (0, 1, 2, 3...). The
**global position** represents the event's location in EventStoreDB's global
transaction log and consists of two coordinates: the `commit` position (where
the transaction was committed in the log) and the `prepare` position (where the
transaction was initially prepared).

These positioning identifiers are essential for reading operations, as they
allow you to specify exactly where to start reading from within a stream or
across the entire event store.

## Reading from a stream

You can read all the events or a sample of the events from individual streams,
starting from any position in the stream, and can read either forward or
backward. It is only possible to read events from a single stream at a time. You
can read events from the global event log, which spans across streams. Learn
more about this process in the [Read from `$all`](#reading-from-the-all-stream)
section below.

### Reading forwards

The simplest way to read a stream forwards is to supply a stream name, read
direction, and revision from which to start. The revision can be specified in several ways:

- Use `StreamPosition::Start` to begin from the very beginning of the stream
- Use `StreamPosition::End` to begin from the current end of the stream
- Use `StreamPosition::Position` with a specific revision number (64-bit signed integer)

```rs
let options = ReadStreamOptions::default()
    .position(StreamPosition::Start)
    .forwards();

let mut stream = client.read_stream("some-stream", &options).await?;
```

You can also start reading from a specific revision in the stream:

```rs
let options = ReadStreamOptions::default()
    .position(StreamPosition::Position(10))
    .max_count(20);
let mut stream = client.read_stream("some-stream", &options).await?;
```

You can then iterate synchronously through the result:

```rs
while let Some(event) = stream.next().await? {
    let test_event = event.get_original_event().as_json::<TestEvent>()?;

    println!("Event> {:?}", test_event);
}
```

There are a number of additional arguments you can provide when reading a stream.

#### maxCount

Passing in the max count allows you to limit the number of events that returned.

In the example below, we read a maximum of 10 events from the stream:

```rs
let options = ReadStreamOptions::default()
    .position(StreamPosition::Position(10))
    .max_count(20);
```

#### resolveLinkTos

When using projections to create new events you can set whether the generated events are pointers to existing events. Setting this value to true will tell EventStoreDB to return the event as well as the event linking to it.

```rs
let options = ReadAllOptions::default().resolve_link_tos();
```

#### userCredentials

The credentials used to read the data can be used by the subscription as follows. This will override the default credentials set on the connection.

```rs
let options = ReadStreamOptions::default()
    .position(StreamPosition::Start)
    .authenticated(Credentials::new("admin", "changeit"));

let stream = client.read_stream("some-stream", &options).await;
```

### Reading backwards

In addition to reading a stream forwards, streams can be read backwards. To read all the events backwards, set the `fromRevision` to `END`:

```rs
let options = ReadStreamOptions::default()
    .position(StreamPosition::End)
    .backwards();
let mut stream = client.read_stream("some-stream", &options).await?;
```

:::tip
Read one event backwards to find the last position in the stream.
:::

### Checking if the stream exists

Reading a stream returns a `ReadStream` that you can iterate over. When iterating over events from a non-existent stream, the `next()` method will return a `crate::Error::ResourceNotFound` error.

It is important to handle this error when attempting to iterate a stream that may not exist.

For example:

```rs{13-15}
let options = ReadStreamOptions::default().position(StreamPosition::Position(10));

let mut stream = client.read_stream("order-0", &options).await?;

match stream.next().await {
    Ok(Some(event)) => {
        let test_event = event.get_original_event().as_json::<OrderCreated>()?;
        println!("Event> {:?}", test_event);
    }
    Ok(None) => {
        println!("End of stream reached");
    }
    Err(crate::Error::ResourceNotFound) => {
        println!("Stream does not exist");
    }
    Err(e) => {
        return Err(e);
    }
}
```

## Reading from the $all stream

Reading from the `$all` stream is similar to reading from an individual stream, but please note there are differences. One significant difference is the need to provide admin user account credentials to read from the `$all` stream.  Additionally, you need to provide a transaction log position instead of a stream revision when reading from the `$all` stream.

### Reading forwards

The simplest way to read the `$all` stream forwards is to supply a read
direction and the transaction log position from which you want to start. The
transaction log position can be specified in several ways:

- Use `start` to begin from the very beginning of the transaction log
- Use `end` to begin from the current end of the transaction log  
- Use `fromPosition` with a specific `Position` object containing commit and prepare coordinates

```rs
let options = ReadAllOptions::default()
    .position(StreamPosition::Start)
    .forwards();
```

You can also start reading from a specific position in the transaction log:

```rs
let options = ReadAllOptions::default()
    .position(StreamPosition::Position(Position {
        commit: 1_110,
        prepare: 1_110,
    }));
```

You can then iterate synchronously through the result:

```rs
while let Some(event) = stream.next().await? {
    println!("Event> {:?}", event.get_original_event());
}
```

There are a number of additional arguments you can provide when reading the `$all` stream.

#### maxCount

Passing in the max count allows you to limit the number of events that returned.

In the example below, we read a maximum of 10 events:

```rs
let options = ReadAllOptions::default()
    .position(StreamPosition::Position(10))
    .max_count(20);
```

#### resolveLinkTos

When using projections to create new events you can set whether the generated events are pointers to existing events. Setting this value to true will tell EventStoreDB to return the event as well as the event linking to it.

```rs
let options = ReadAllOptions::default().resolve_link_tos();
```

#### userCredentials
The credentials used to read the data can be used by the subscription as follows. This will override the default credentials set on the connection.

```rs
let options = ReadAllOptions::default()
    .authenticated(Credentials::new("admin", "changeit"))
```

### Reading backwards

In addition to reading the `$all` stream forwards, it can be read backwards. To
read all the events backwards, set the _direction_ to `esdb.Backwards`:

```rs
let options = ReadAllOptions::default().position(StreamPosition::End);

let mut stream = client.read_all(&options).await?;
```

:::tip
Read one event backwards to find the last position in the `$all` stream.
:::

### Handling system events

EventStoreDB will also return system events when reading from the `$all` stream. In most cases you can ignore these events.

All system events begin with `$` or `$$` and can be easily ignored by checking the `event_type` property.

```rs
let mut stream = client.read_all(&Default::default()).await?;

while let Some(event) = stream.next().await? {
    if event.get_original_event().event_type.starts_with("$") {
        continue;
    }

    println!("Event> {:?}", event.get_original_event());
}
```