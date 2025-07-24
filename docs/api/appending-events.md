---
order: 2
---

# Appending events

When you start working with EventStoreDB, your application streams are empty. The first meaningful operation is to add one or more events to the database using this API.

::: tip
Check the [Getting Started](getting-started.md) guide to learn how to configure and use the client SDK.
:::

## Append your first event

The simplest way to append an event to EventStoreDB is to create an `EventData` object and call `append_to_stream` method.

```rs{7-11}
let data = OrderCreated {
    order_id: Uuid::new_v4().to_string(),
};

let event = EventData::json("OrderCreated", &data)?.id(Uuid::new_v4());

let options = AppendToStreamOptions::default().expected_revision(ExpectedRevision::NoStream);

let _ = client
    .append_to_stream("order-123", &options, event)
    .await?;
```

`append_to_stream` takes a collection or a single object that can be serialized in JSON or binary format, which allows you to save more than one event in a single batch.
 
Outside the example above, other options exist for dealing with different scenarios. 

::: tip
If you are new to Event Sourcing, please study the [Handling concurrency](#handling-concurrency) section below.
:::

## Working with EventData

Events appended to EventStoreDB must be wrapped in an `EventData` object. This allows you to specify the event's content, the type of event, and whether it's in JSON format. In its simplest form, you need three arguments: **eventId**, **eventType**, and **eventData**.

### EventID

This takes the format of a `UUID` and is used to uniquely identify the event you are trying to append. If two events with the same `UUID` are appended to the same stream in quick succession, EventStoreDB will only append one of the events to the stream. 

For example, the following code will only append a single event:

```rs{12-15}
let data = OrderCreated {
    order_id: Uuid::new_v4().to_string(),
};

let event = EventData::json("OrderCreated", &data)?.id(Uuid::new_v4());
let options = AppendToStreamOptions::default();

let _ = client
    .append_to_stream("order-123", &options, event.clone())
    .await?;

// attempt to append the same event again
let _ = client
    .append_to_stream("order-123", &options, event)
    .await?;
```

### EventType

Each event should be supplied with an event type. This unique string is used to identify the type of event you are saving. 

It is common to see the explicit event code type name used as the type as it makes serialising and de-serialising of the event easy. However, we recommend against this as it couples the storage to the type and will make it more difficult if you need to version the event at a later date.

### Data

Representation of your event data. It is recommended that you store your events as JSON objects. This allows you to take advantage of all of EventStoreDB's functionality, such as projections. That said, you can save events using whatever format suits your workflow. Eventually, the data will be stored as encoded bytes.

### Metadata

Storing additional information alongside your event that is part of the event itself is standard practice. This can be correlation IDs, timestamps, access information, etc. EventStoreDB allows you to store a separate byte array containing this information to keep it separate.

### ContentType

The content type indicates whether the event is stored as JSON or binary format. You can choose between `esdb.ContentTypeJson` and `esdb.ContentTypeBinary` when creating your `EventData` object. 

## Handling concurrency

When appending events to a stream, you can supply a *stream state*. Your client uses this to inform EventStoreDB of the state or version you expect the stream to be in when appending an event. If the stream isn't in that state, an exception will be thrown. 

For example, if you try to append the same record twice, expecting both times that the stream doesn't exist, you will get an exception on the second:

```rs
let data = OrderCreated {
    order_id: "1"
};

let event = EventData::json("OrderCreated", &data)?.id(Uuid::new_v4());
let options = AppendToStreamOptions::default().expected_revision(ExpectedRevision::NoStream);

let _ = client
    .append_to_stream("order-456", &options, event)
    .await?;

let data = OrderCreated {
    order_id: "2"
};

let event = EventData::json("OrderCreated", &data)?.id(Uuid::new_v4());

let _ = client
    .append_to_stream("order-456", &options, event)
    .await?;
```

There are several available expected revision options: 
- `Any` - No concurrency check
- `NoStream` - Stream should not exist
- `StreamExists` - Stream should exist
- `Exact` - Stream should be at specific revision

This check can be used to implement optimistic concurrency. When retrieving a
stream from EventStoreDB, note the current version number. When you save it back,
you can determine if somebody else has modified the record in the meantime.

```rs{8-13,22,36-38}
struct OrderUpdated {
    pub order_id: String,
    pub status: String,
}

let options = ReadStreamOptions::default().position(StreamPosition::End);

let last_event = client
    .read_stream("order-789", &options)
    .await?
    .next()
    .await?
    .expect("the stream to at least exist.");

let data = OrderUpdated {
    order_id: "1".to_string(),
    status: "processing".to_string(),
};

let event = EventData::json("OrderUpdated", &data)?.id(Uuid::new_v4());
let options = AppendToStreamOptions::default().expected_revision(ExpectedRevision::StreamRevision(
    last_event.get_original_event().revision,
));

let _ = client
    .append_to_stream("order-789", &options, event)
    .await?;

let data = OrderUpdated {
    order_id: "2".to_string(),
    status: "shipped".to_string(),
};

let event = EventData::json("OrderUpdated", &data)?.id(Uuid::new_v4());

let _ = client
    .append_to_stream("order-789", &options, event)
    .await?;
```

## User credentials

You can provide user credentials to append the data as follows. This will override the default credentials set on the connection.

```rs
let options =
    AppendToStreamOptions::default().authenticated(Credentials::new("admin", "changeit"));

let _ = client
    .append_to_stream("order-123", &options, event)
    .await?;
```