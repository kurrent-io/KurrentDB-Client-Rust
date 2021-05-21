use crate::{Credentials, Position, ReadDirection, StreamPosition};

#[derive(Clone)]
pub struct ReadAllOptions {
    pub(crate) credentials: Option<Credentials>,
    pub(crate) direction: ReadDirection,
    pub(crate) position: StreamPosition<Position>,
    pub(crate) resolve_link_tos: bool,
}

impl Default for ReadAllOptions {
    fn default() -> Self {
        Self {
            credentials: None,
            direction: ReadDirection::Forward,
            position: StreamPosition::Start,
            resolve_link_tos: false,
        }
    }
}

impl ReadAllOptions {
    /// Asks the command to read forward (toward the end of the stream).
    /// That's the default behavior.
    pub fn forwards(self) -> Self {
        Self {
            direction: ReadDirection::Forward,
            ..self
        }
    }

    /// Asks the command to read backward (toward the begining of the stream).
    pub fn backwards(self) -> Self {
        Self {
            direction: ReadDirection::Backward,
            ..self
        }
    }

    /// Performs the command with the given credentials.
    pub fn authenticated(self, value: Credentials) -> Self {
        Self {
            credentials: Some(value),
            ..self
        }
    }

    /// Starts the read at the given position. Default `StreamPosition::Start`
    pub fn position(self, position: StreamPosition<Position>) -> Self {
        match position {
            StreamPosition::Start => Self {
                position,
                direction: ReadDirection::Forward,
                ..self
            },

            StreamPosition::End => Self {
                position,
                direction: ReadDirection::Backward,
                ..self
            },

            StreamPosition::Position(_) => Self { position, ..self },
        }
    }

    /// When using projections, you can have links placed into another stream.
    /// If you set `true`, the server will resolve those links and will return
    /// the event that the link points to. Default: [NoResolution](../types/enum.LinkTos.html).
    pub fn resolve_link_tos(self) -> Self {
        Self {
            resolve_link_tos: true,
            ..self
        }
    }
}
