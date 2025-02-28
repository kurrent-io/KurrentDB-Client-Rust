use crate::StreamState;
use eventstore_macros::options;

options! {
    #[derive(Clone)]
    /// Options of the tombstone stream command.
    pub struct TombstoneStreamOptions {
        pub(crate) version: StreamState,
    }
}

impl Default for TombstoneStreamOptions {
    fn default() -> Self {
        Self {
            version: StreamState::Any,
            common_operation_options: Default::default(),
        }
    }
}

impl TombstoneStreamOptions {
    /// Asks the server to check that the stream receiving the event is at
    /// the given expected version. Default: `ExpectedVersion::Any`.
    pub fn expected_revision(self, version: StreamState) -> Self {
        Self { version, ..self }
    }
}
