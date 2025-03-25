# KurrentDB Rust Client
[![Crates.io][crates-badge]][crates-url]
[![Crates.io][crates-download]][crates-url]
[![Build Status][ci-badge]][ci-url]
![Discord](https://img.shields.io/discord/415421715385155584.svg)
![Crates.io](https://img.shields.io/crates/l/kurrentdb.svg)

[crates-badge]: https://img.shields.io/crates/v/kurrentdb.svg
[crates-download]: https://img.shields.io/crates/d/kurrentdb.svg
[crates-url]: https://crates.io/crates/kurrentdb
[ci-badge]: https://github.com/EventStore/EventStoreDB-Client-Rust/workflows/CI/badge.svg
[ci-url]: https://github.com/EventStore/EventStoreDB-Client-Rust/actions

[Documentation](https://docs.rs/kurrentdb)

Official Rust [KurrentDB rust gRPC] gRPC Client.

[KurrentDB] is the event-native database, where business events are immutably stored and streamed. Designed for event-sourced, event-driven, and microservices architectures.

## KurrentDB Server Compatibility
This client is compatible with version `20.6.1` upwards and works on Linux, MacOS and Windows.


Server setup instructions can be found here [KurrentDB Docs], follow the docker setup for the simplest configuration.

# Example

```rust
use kurrentdb::{ Client, EventData };
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Foo {
    is_rust_a_nice_language: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Creates a client settings for a single node configuration.
    let settings = "kurrentdb://admin:changeit@localhost:2113".parse()?;
    let client = Client::new(settings)?;

    let payload = Foo {
        is_rust_a_nice_language: true,
    };

    // It is not mandatory to use JSON as a data format however KurrentDB
    // provides great additional value if you do so.
    let evt = EventData::json("language-poll", &payload)?;

    client
        .append_to_stream("language-stream", &Default::default(), evt)
        .await?;

    let mut stream = client
        .read_stream("language-stream", &Default::default())
        .await?;

    while let Some(event) = stream.next().await? {
        let event = event.get_original_event()
          .as_json::<Foo>()?;

        // Do something productive with the result.
        println!("{:?}", event);
    }

    Ok(())
}
```

## Support

Information on support can be found here: [KurrentDB Support]

## Documentation

Documentation for KurrentDB can be found here: [KurrentDB Docs]

Bear in mind that this client is not yet properly documented. We are working hard on a new version of the documentation.

## Communities

- [Discuss](https://discuss.kurrent.io/)
- [Discord (Kurrent)](https://discord.gg/Phn9pmCw3t)

[KurrentDB]: https://kurrent.io/
[KurrentDB rust gRPC]: https://developers.kurrent.io/clients/grpc/getting-started?codeLanguage=Rust
[kurrent docs]: https://developers.kurrent.io/latest.html
[kurrent discuss]: https://discuss.kurrent.io/
[kurrent support]: https://kurrent.io/support/
