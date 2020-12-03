0.9.7
=====
* Update DNS discovery so it works properly with EventStore Cloud.

0.9.6
=====
* Add Debug instance for `SubEvent`.
* No longer implicitly use `PersistentSubscriptionSettings` default values on both persistent subscription creation and update.

0.9.5
=====
* Expose subscription lifecyle events.
* Improve FilterConf usability.

0.9.4
=====
* Use consistent names across all official clients.
* Fix documentation.

0.9.3
=====
* Add prelude module.
* Re-export `FilterConf`.

0.9.2
=====
* Add DNS record type selection in connection string. DNS A queries are done by default now. Use to be SRV.
* Export `ConnectionSettingsParseError` type.
* `read_stream` returns `ReadResult::StreamNotFound` explicitly.
* Add `Position::end()`

0.9.1
=====
* Add more documentation.
* More flexible `EventData::json` and `EventData::binary`.

0.9
=====
* gRPC is now the only supported interface.
* Delete TCP interface.
* Add `read_through` function when reading a stream (`$all` included).
* Support latest `message_timeout_ms` and `checkpoint_after_ms` persistent.proto changes.
* Improve user experience.
* Implement cluster mode connection.

0.8.1
=====
* Bump protobuf version.
* Bump tonic version.
* Make Connection cloneable.
* Support DNS discovery for the TCP API.

0.8.0
=====
* Implement secure connection for the TCP API.
* Support 20.6 stable version (behind 20.6).

0.7.5
=====
* Make connection timeout period configurable.
* Don't panic on second call if server is off.
* Migrate gRPC interface to ES6 preview 3 version.

0.7.4
=====
* No change, only updated website link.

0.7.3
=====
* Expose subscription events so the user can be notified when a subscription has been confirmed or dropped by the server.
* Upgrade to uuid 0.8.* version.
* Fix memory leak in registry when dealing with subscriptions.
* Allow user to convert UUID to GUID when sending events.
* Fix persistent subscription regression when ack/nak.

0.7.2
=====
* Idiomatic streaming interface for subscriptions and batch reads (tcp API).
* Fix UUID/GUID serialization.

0.7.1
=====
* Ask Docs.rs to also build documentation for es6 module.

0.7.0
=====
* Expose ES 6 gRPC interface.

0.6.0
=====
* Move to tokio 0.2

0.5.1
=====
* Pinned `protobuf` to 2.8.1 version.

0.5.0
=====
* Support Rust 1.39
* Remove protobuf::Chars from public API.

0.4.5
=====
* Update persistent subscription default settings.
* Internal connection refactoring.
* Implement `iterate_over_batch`.

0.4.4
=====
* Remove debugging leftovers.
* Add `Pinned` system consumer strategy.

0.4.3
=====
* Fix reading a deleted stream event in $streams stream, causing a read command to abort.

0.4.2
=====
* Fix compiler warnings.
* Bump dependencies version.

0.4.1
=====
* Fix stream metadata and ACL JSON (de)serialization.

0.4.0
=====
* Implement cluster-mode connection.
* Internal refactoring.
* `start` and `start_with_runtime` are renamed `single_node_connection` and `single_node_connection_with_runtime`.

0.3.0
=====
* Migrate `iterate_over` from iterator to asynchronous stream.

0.2.4
=====
* Fix possible connection issues if Authentication or Identification processes take too long to complete.

0.2.3
=====
* Remove an unnecessary OS thread.
* Implement `ConnectionBuilder::start_with_runtime` to use an existing tokio runtime.
* Fix rare issue where the user sends a command before the connection is confirmed with the
  server, causing that operation to be sent only after a `operation timeout` time.
* No longer terminate the connection in case of identification timeout.

0.2.2
=====
* Implement stream streaming ($all included).

0.2.1
=====
* BUGFIX: Fix next event number for stream reads.

0.2.0
=====
* Simplify public eventstore module.
* Move to a typeful representation of `resolve_link_tos` setting.
* Implement connection state-machine graceful exit.
* Introduce new connection api.

0.1.3
=====
* Migrate to `uuid` 0.7.
* Move to tokio multithreaded runtime.
