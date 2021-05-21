use crate::{Credentials, PersistentSubscriptionSettings, StreamPosition};

#[derive(Clone)]
pub struct PersistentSubscriptionOptions {
    pub(crate) credentials: Option<Credentials>,
    pub(crate) setts: PersistentSubscriptionSettings,
    pub(crate) revision: StreamPosition<u64>,
}

impl Default for PersistentSubscriptionOptions {
    fn default() -> Self {
        Self {
            credentials: None,
            setts: Default::default(),
            revision: StreamPosition::End,
        }
    }
}

impl PersistentSubscriptionOptions {
    /// Performs the command with the given credentials.
    pub fn authenticated(self, value: Credentials) -> Self {
        Self {
            credentials: Some(value),
            ..self
        }
    }

    /// Applies the specified persistent subscription settings.
    pub fn settings(self, setts: PersistentSubscriptionSettings) -> Self {
        Self { setts, ..self }
    }

    /// Where the subscription should start from (event number).
    pub fn revision(self, revision: StreamPosition<u64>) -> Self {
        Self { revision, ..self }
    }
}

#[derive(Clone)]
pub struct DeletePersistentSubscriptionOptions {
    pub(crate) credentials: Option<Credentials>,
}

impl Default for DeletePersistentSubscriptionOptions {
    fn default() -> Self {
        Self { credentials: None }
    }
}

impl DeletePersistentSubscriptionOptions {
    /// Performs the command with the given credentials.
    pub fn authenticated(self, value: Credentials) -> Self {
        Self {
            credentials: Some(value),
        }
    }
}

#[derive(Clone)]
pub struct ConnectToPersistentSubscription {
    pub(crate) credentials: Option<Credentials>,
    pub(crate) batch_size: usize,
}

impl Default for ConnectToPersistentSubscription {
    fn default() -> Self {
        Self {
            credentials: None,
            batch_size: 10,
        }
    }
}

impl ConnectToPersistentSubscription {
    /// Performs the command with the given credentials.
    pub fn authenticated(self, creds: Credentials) -> Self {
        Self {
            credentials: Some(creds),
            ..self
        }
    }

    /// The buffer size to use  for the persistent subscription.
    pub fn batch_size(self, batch_size: usize) -> Self {
        Self { batch_size, ..self }
    }
}
