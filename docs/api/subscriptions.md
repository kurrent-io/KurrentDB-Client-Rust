---
order: 4
---

# Catch-up Subscriptions

Subscriptions allow you to subscribe to a stream and receive notifications about new events added to the stream. You provide an event handler and an optional starting point to the subscription. The handler is called for each event from the starting point onward.

If events already exist, the handler will be called for each event one by one until it reaches the end of the stream. The server will then notify the handler whenever a new event appears.

:::tip
Check the [Getting Started](getting-started.md) guide to learn how to configure and use the client SDK.
:::

## Basic Subscriptions

You can subscribe to a single stream or to `$all` to process all events in the database.

**Stream subscription:**

```rs
let mut sub = client
    .subscribe_to_stream("order-123", &Default::default())
    .await;

loop {
    let event = sub.next_subscription_event().await?;
    match event {
        SubscriptionEvent::EventAppeared(event) => {
            let stream_id = event.get_original_stream_id();
            let revision = event.get_original_event().revision;

            // handle the event
        }

        SubscriptionEvent::CaughtUp(position) => {
        }

        _ => {}
    }
}
```

**`$all` subscription:**

```rs
let mut sub = client.subscribe_to_all(&Default::default()).await;

loop {
    let event = sub.next_subscription_event().await?;
    match event {
        SubscriptionEvent::EventAppeared(event) => {
            let stream_id = event.get_original_stream_id();
            let revision = event.get_original_event().revision;

            // handle the event
        }

        SubscriptionEvent::CaughtUp(position) => {
        }

        _ => {}
    }
}
```

When you subscribe to a stream with link events (e.g., `$ce` category stream), set `resolve_link_tos` to `true`.

## Subscribing from a Position

Both stream and `$all` subscriptions accept a starting position if you want to read from a specific point onward. If events already exist at the position you subscribe to, they will be read on the server side and sent to the subscription.

Once caught up, the server will push any new events received on the streams to the client. There is no difference between catching up and live on the client side.

::: warning
The positions provided to the subscriptions are exclusive. You will only receive the next event after the subscribed position.
:::

**Stream from specific position:**

To subscribe to a stream from a specific position, provide a stream position (`StreamPosition::Start`, `StreamPosition::End` or a 64-bit signed integer representing the revision number):

```rs
let options = SubscribeToStreamOptions::default().start_from(StreamPosition::Position(20));

client.subscribe_to_stream("order-123", &options).await;
```

**`$all` from specific position:**

For the `$all` stream, provide a `Position` structure with prepare and commit positions:

```rs
let options = SubscribeToAllOptions::default().position(StreamPosition::Position(Position {
    commit: 1_056,
    prepare: 1_056,
}));

client.subscribe_to_all(&options).await;
```

**Live updates only:**

Subscribe to the end of a stream to get only new events:

```rs
// Stream
let options = SubscribeToStreamOptions::default().start_from(StreamPosition::End);

// $all
let options = SubscribeToAllOptions::default().position(StreamPosition::End);
```

## Resolving link-to events

Link-to events point to events in other streams in KurrentDB. These are
generally created by projections such as the `$by_event_type` projection which
links events of the same event type into the same stream. This makes it easier
to look up all events of a specific type.

::: tip
[Filtered subscriptions](subscriptions.md#server-side-filtering) make it easier
and faster to subscribe to all events of a specific type or matching a prefix.
:::

When reading a stream you can specify whether to resolve link-to's. By default,
link-to events are not resolved. You can change this behaviour by setting the
`resolve_link_tos` parameter to `true`:

```rs
let options = SubscribeToStreamOptions::default()
    .start_from(StreamPosition::Start)
    .resolve_link_tos();
```

## Subscription Drops and Recovery

When a subscription stops or experiences an error, it will be dropped. The
subscription provides an `error` event in the `StreamSubscription` Node.js
readable stream, which will get called when the subscription breaks.

The `error` event allows you to inspect the reason why the
subscription dropped, as well as any exceptions that occurred.

Bear in mind that a subscription can also drop because it is slow. The server
tried to push all the live events to the subscription when it is in the live
processing mode. If the subscription gets the reading buffer overflow and won't
be able to acknowledge the buffer, it will break.

### Handling Dropped Subscriptions

An application, which hosts the subscription, can go offline for some time for
different reasons. It could be a crash, infrastructure failure, or a new version
deployment. The retry logic is built into the SDK to handle such cases and will
automatically reconnect the subscription when the application is back online. 

```rs
let retry = RetryOptions::default().retry_forever();
let options = SubscribeToStreamOptions::default().retry_options(retry);

let mut stream = client.subscribe_to_stream("some-stream", &options).await;

loop {
    let event = stream.next().await?;
}
```

## Handling Subscription State Changes

::: info EventStoreDB 23.10.0+
This feature requires EventStoreDB version 23.10.0 or later.
:::

When a subscription processes historical events and reaches the end of the
stream, it transitions from "catching up" to "live" mode. You can detect this
transition using the `caughtUp` event on the subscription. 

```rs{15-17}
let mut sub = client
    .subscribe_to_stream("order-123", &Default::default())
    .await;

loop {
    let event = sub.next_subscription_event().await?;
    match event {
        SubscriptionEvent::EventAppeared(event) => {
            let stream_id = event.get_original_stream_id();
            let revision = event.get_original_event().revision;

            // handle the event
        }

        SubscriptionEvent::CaughtUp(position) => {
            // Handle the transition to live mode
        }

        _ => {}
    }
}
```

::: tip
The `CaughtUp` event is only emitted when transitioning from catching up to live mode. If you subscribe from the end of a stream, you'll immediately be in live mode and this callback will be called right away.
:::


## User credentials

The user creating a subscription must have read access to the stream it's
subscribing to, and only admin users may subscribe to `$all` or create filtered
subscriptions.

The code below shows how you can provide user credentials for a subscription.
When you specify subscription credentials explicitly, it will override the
default credentials set for the client. If you don't specify any credentials,
the client will use the credentials specified for the client, if you specified
those.

```rs
let options =
    SubscribeToAllOptions::default().authenticated(Credentials::new("admin", "changeit"));
```

## Server-side Filtering

KurrentDB allows you to filter events while subscribing to the `$all` stream to only receive the events you care about. You can filter by event type or stream name using a regular expression or a prefix. Server-side filtering is currently only available on the `$all` stream.

::: tip
Server-side filtering was introduced as a simpler alternative to projections. You should consider filtering before creating a projection to include the events you care about.
:::

**Basic filtering:**

```rs
let filter = SubscriptionFilter::on_stream_name()
    .add_prefix("test-")
    .add_prefix("other-");

let options = SubscribeToAllOptions::default().filter(filter);

client.subscribe_to_all(&options).await;
```

### Filtering out system events

System events are prefixed with `$` and can be filtered out when subscribing to `$all`:

```rs
let filter = SubscriptionFilter::on_event_type().exclude_system_events();
let options = SubscribeToAllOptions::default().filter(filter);
```

### Filtering by event type

**By prefix:**

```rs
let filter = SubscriptionFilter::on_event_type().add_prefix("customer-");
let options = SubscribeToAllOptions::default().filter(filter);
```

**By regular expression:**

```rs
let filter = SubscriptionFilter::on_event_type().regex("^user|^company");
let options = SubscribeToAllOptions::default().filter(filter);
```

### Filtering by stream name

**By prefix:**

```rs
let filter = SubscriptionFilter::on_stream_name().add_prefix("user-");
let options = SubscribeToAllOptions::default().filter(filter);
```

**By regular expression:**

```rs
let filter = SubscriptionFilter::on_event_type().regex("/^[^\\$].*/");
let options = SubscribeToAllOptions::default().filter(filter);
```

## Checkpointing

When a catch-up subscription is used to process an `$all` stream containing many events, the last thing you want is for your application to crash midway, forcing you to restart from the beginning.

### What is a checkpoint?

A checkpoint is the position of an event in the `$all` stream to which your application has processed. By saving this position to a persistent store (e.g., a database), it allows your catch-up subscription to:

- Recover from crashes by reading the checkpoint and resuming from that position
- Avoid reprocessing all events from the start

To create a checkpoint, store the event's commit or prepare position.

::: warning
If your database contains events created by the legacy TCP client using the [transaction feature](https://docs.kurrent.io/clients/tcp/dotnet/21.2/appending.html#transactions), you should store both the commit and prepare positions together as your checkpoint.
:::

### Updating checkpoints at regular intervals

The SDK provides a way to notify your application after processing a configurable number of events. This allows you to periodically save a checkpoint at regular intervals.

```rs
loop {
    let event = sub.next_subscription_event().await?;
    match event {
        SubscriptionEvent::EventAppeared(event) => {
            let stream_id = event.get_original_stream_id();
            let revision = event.get_original_event().revision;

            println!("Received event {}@{}", revision, stream_id);
        }

        SubscriptionEvent::Checkpoint(position) => {
            // Save commit position to a persistent store as a checkpoint
            println!("checkpoint taken at {}", position.commit);
        }

        _ => {}
    }
}
```

By default, the checkpoint notification is sent after every 32 non-system events processed from $all.

### Configuring the checkpoint interval

You can adjust the checkpoint interval to change how often the client is notified. 

```rs
let filter = SubscriptionFilter::on_event_type().regex("/^[^\\$].*/");
let options = SubscribeToAllOptions::default().filter(filter);

let mut sub = client.subscribe_to_all(&options).await;
```

By configuring this parameter, you can balance between reducing checkpoint overhead and ensuring quick recovery in case of a failure.

::: info
The checkpoint interval parameter configures the database to notify the client after `n` * 32 number of events where `n` is defined by the parameter.

For example:
- If `n` = 1, a checkpoint notification is sent every 32 events.
- If `n` = 2, the notification is sent every 64 events.
- If `n` = 3, it is sent every 96 events, and so on.
:::
