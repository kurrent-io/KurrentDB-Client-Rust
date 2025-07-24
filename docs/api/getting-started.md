---
order: 1
---

# Getting started

This guide will help you get started with EventStoreDB in your Java application.
It covers the basic steps to connect to EventStoreDB, create events, append them
to streams, and read them back.

## Required packages

Add the following crates to your project by including them in the dependencies
list located in your project's `Cargo.toml` file:

```toml
[dependencies]
serde = "1.0.188"
futures = "0.3.28"
tokio = {version = "1.32.0", features = ["full"]}

[dependencies.eventstore]
version = "4.0.0"
```

## Connecting to EventStoreDB

To connect your application to EventStoreDB, you need to configure and create a client instance.

::: tip Insecure clusters
The recommended way to connect to EventStoreDB is using secure mode (which is
the default). However, if your EventStoreDB instance is running in insecure
mode, you must explicitly set `tls=false` in your connection string or client configuration.
:::

EventStoreDB uses connection strings to configure the client connection. The connection string supports two protocols:

- **`esdb://`** - for connecting directly to specific node endpoints (single node or multi-node cluster with explicit endpoints)
- **`esdb+discover://`** - for connecting using cluster discovery via DNS or gossip endpoints

When using `esdb://`, you specify the exact endpoints to connect to. The client will connect directly to these endpoints. For multi-node clusters, you can specify multiple endpoints separated by commas, and the client will query each node's Gossip API to get cluster information, then picks a node based on the URI's node preference.

With `esdb+discover://`, the client uses cluster discovery to find available nodes. This is particularly useful when you have a DNS A record pointing to cluster nodes or when you want the client to automatically discover the cluster topology.

::: info Gossip support
Since version 22.10, esdb supports gossip on single-node deployments, so
`esdb+discover://` can be used for any topology, including single-node setups.
:::

For cluster connections using discovery, use the following format:

```
esdb+discover://admin:changeit@cluster.dns.name:2113
```

Where `cluster.dns.name` is a DNS `A` record that points to all cluster nodes.

For direct connections to specific endpoints, you can specify individual nodes:

```
esdb://admin:changeit@node1.dns.name:2113,node2.dns.name:2113,node3.dns.name:2113
```

Or for a single node:

```
esdb://admin:changeit@localhost:2113
```

There are a number of query parameters that can be used in the connection string to instruct the cluster how and where the connection should be established. All query parameters are optional.

| Parameter             | Accepted values                                   | Default  | Description                                                                                                                                    |
|-----------------------|---------------------------------------------------|----------|------------------------------------------------------------------------------------------------------------------------------------------------|
| `tls`                 | `true`, `false`                                   | `true`   | Use secure connection, set to `false` when connecting to a non-secure server or cluster.                                                       |
| `connectionName`      | Any string                                        | None     | Connection name                                                                                                                                |
| `maxDiscoverAttempts` | Number                                            | `10`     | Number of attempts to discover the cluster.                                                                                                    |
| `discoveryInterval`   | Number                                            | `100`    | Cluster discovery polling interval in milliseconds.                                                                                            |
| `gossipTimeout`       | Number                                            | `5`      | Gossip timeout in seconds, when the gossip call times out, it will be retried.                                                                 |
| `nodePreference`      | `leader`, `follower`, `random`, `readOnlyReplica` | `leader` | Preferred node role. When creating a client for write operations, always use `leader`.                                                         |
| `tlsVerifyCert`       | `true`, `false`                                   | `true`   | In secure mode, set to `true` when using an untrusted connection to the node if you don't have the CA file available. Don't use in production. |
| `tlsCaFile`           | String, file path                                 | None     | Path to the CA file when connecting to a secure cluster with a certificate that's not signed by a trusted CA.                                  |
| `defaultDeadline`     | Number                                            | None     | Default timeout for client operations, in milliseconds. Most clients allow overriding the deadline per operation.                              |
| `keepAliveInterval`   | Number                                            | `10`     | Interval between keep-alive ping calls, in seconds.                                                                                            |
| `keepAliveTimeout`    | Number                                            | `10`     | Keep-alive ping call timeout, in seconds.                                                                                                      |
| `userCertFile`        | String, file path                                 | None     | User certificate file for X.509 authentication.                                                                                                |
| `userKeyFile`         | String, file path                                 | None     | Key file for the user certificate used for X.509 authentication.                                                                               |

When connecting to an insecure instance, specify `tls=false` parameter. For example, for a node running locally use `esdb://localhost:2113?tls=false`. Note that usernames and passwords aren't provided there because insecure deployments don't support authentication and authorisation.

## Creating a client

First, create a client and get it connected to the database.

```rs
let settings = "esdb://localhost:2113?tls=false&tlsVerifyCert=false".parse()?;
let client = Client::new(settings)?;
```

The client instance can be used as a singleton across the whole application. It doesn't need to open or close the connection.

## Creating an event

You can write anything to EventStoreDB as events. The client needs a byte array as the event payload. Normally, you'd use a serialized object, and it's up to you to choose the serialization method.

The code snippet below creates an event object instance, serializes it, and adds it as a payload to the `EventData` structure, which the client can then write to the database.

```rs
let event = OrderCreated {
    order_id: Uuid::new_v4().to_string(),
};

let event_data = EventData::json("OrderCreated", &event)?.id(Uuid::new_v4());
```

## Appending events

Each event in the database has its own unique identifier (UUID). The database uses it to ensure idempotent writes, but it only works if you specify the stream revision when appending events to the stream.

In the snippet below, we append the event to the stream `orders`.

```rs
client
    .append_to_stream("order-123", &Default::default(), event_data)
    .await?;
```

Here we are appending events without checking if the stream exists or if the stream version matches the expected event version. See more advanced scenarios in [appending events documentation](./appending-events.md).

## Reading events

Finally, we can read events back from the stream.

```rs
let options = ReadStreamOptions::default().max_count(10);
let mut stream = client.read_stream("order-123", &options).await?;

while let Some(event) = stream.next().await? {
  // Handle the event
}
```