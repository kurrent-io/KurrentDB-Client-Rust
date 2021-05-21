//! Official Rust [EventStoreDB] gRPC Client.
//!
//! [EventStoreDB] is an open-source database built from the ground up for Event Sourcing, with Complex Event Processing in Javascript.
//!
//! ## EventStoreDB Server Compatibility
//! This client is compatible with version `20.6.1` upwards and works on Linux, MacOS and Windows.
//!
//!
//! Server setup instructions can be found here [EventStoreDB Docs], follow the docker setup for the simplest configuration.
//!
//! # Example
//!
//! ```no_run
//! use eventstore::{ All, Client, EventData, ReadResult };
//! use futures::stream::TryStreamExt;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize, Debug)]
//! struct Foo {
//!     is_rust_a_nice_language: bool,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!
//!     // Creates a client settings for a single node configuration.
//!     let settings = "esdb://admin:changeit@localhost:2113".parse()?;
//!     let client = Client::create(settings).await?;
//!
//!     let payload = Foo {
//!         is_rust_a_nice_language: true,
//!     };
//!
//!     // It is not mandatory to use JSON as a data format however EventStoreDB
//!     // provides great additional value if you do so.
//!     let evt = EventData::json("language-poll", &payload)?;
//!
//!     let _ = client
//!         .append_to_stream("language-stream", &Default::default(), evt)
//!         .await?;
//!
//!     let result = client
//!         .read_stream("language-stream", &Default::default(), All)
//!         .await?;
//!
//!     if let ReadResult::Ok(mut stream) = result {
//!         while let Some(event) = stream.try_next().await? {
//!             let event = event.get_original_event()
//!                     .as_json::<Foo>()?;
//!
//!             // Do something productive with the result.
//!             println!("{:?}", event);
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//! [EventStoreDB]: https://eventstore.com/
//! [eventstoredb docs]: https://developers.eventstore.com/server/20.6/server/installation/
#[macro_use]
extern crate log;

mod client;
mod commands;
mod event_store;
mod gossip;
mod grpc;
mod options;
mod private;
mod types;

pub(crate) mod google {
    pub mod rpc {
        pub use super::super::event_store::generated::google_rpc::*;
    }
}

// pub(crate) use event_store::generated::client::*;

pub use client::Client;
pub use commands::{SubscriptionRead, SubscriptionWrite};
pub use grpc::{ClientSettings, ClientSettingsParseError};
pub use options::append_to_stream::*;
pub use options::delete_stream::*;
pub use options::persistent_subscription::*;
pub use options::read_all::*;
pub use options::read_stream::*;
pub use options::retry::*;
pub use options::subscribe_to_all::*;
pub use options::subscribe_to_stream::*;
pub use types::*;

pub mod prelude {
    pub use crate::client::Client;
    pub use crate::commands::{SubscriptionRead, SubscriptionWrite};
    pub use crate::grpc::{ClientSettings, ClientSettingsParseError};
    pub use crate::options::append_to_stream::*;
    pub use crate::options::delete_stream::*;
    pub use crate::options::persistent_subscription::*;
    pub use crate::options::read_all::*;
    pub use crate::options::read_stream::*;
    pub use crate::options::retry::*;
    pub use crate::options::subscribe_to_all::*;
    pub use crate::options::subscribe_to_stream::*;
    pub use crate::types::*;
}
