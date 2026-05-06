use kurrentdb_macros::options;

/// Selects which projection engine the server should use when creating a
/// continuous projection. `V1` is the default and is supported by every server
/// version. `V2` requires a server that supports the next-generation engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectionEngineVersion {
    #[default]
    V1,
    V2,
}

impl ProjectionEngineVersion {
    pub(crate) fn as_i32(self) -> i32 {
        match self {
            ProjectionEngineVersion::V1 => 1,
            ProjectionEngineVersion::V2 => 2,
        }
    }
}

options! {
    #[derive(Clone, Default)]
    pub struct CreateProjectionOptions {
        pub(crate) track_emitted_streams: bool,
        pub(crate) emit: bool,
        pub(crate) engine_version: ProjectionEngineVersion,
    }
}

impl CreateProjectionOptions {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn track_emitted_streams(self, track_emitted_streams: bool) -> Self {
        Self {
            track_emitted_streams,
            ..self
        }
    }

    pub fn emit(self, emit: bool) -> Self {
        Self { emit, ..self }
    }

    pub fn engine_version(self, engine_version: ProjectionEngineVersion) -> Self {
        Self {
            engine_version,
            ..self
        }
    }
}

options! {
    #[derive(Clone, Default)]
    pub struct UpdateProjectionOptions {
        pub(crate) emit: Option<bool>,
    }
}

impl UpdateProjectionOptions {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn emit(self, emit: bool) -> Self {
        Self {
            emit: Some(emit),
            ..self
        }
    }
}

options! {
    #[derive(Clone, Default)]
    pub struct DeleteProjectionOptions {
        pub(crate) delete_emitted_streams: bool,
        pub(crate) delete_state_stream: bool,
        pub(crate) delete_checkpoint_stream: bool,
    }
}

impl DeleteProjectionOptions {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn delete_emitted_streams(self, delete_emitted_streams: bool) -> Self {
        Self {
            delete_emitted_streams,
            ..self
        }
    }

    pub fn delete_state_stream(self, delete_state_stream: bool) -> Self {
        Self {
            delete_state_stream,
            ..self
        }
    }

    pub fn delete_checkpoint_stream(self, delete_checkpoint_stream: bool) -> Self {
        Self {
            delete_checkpoint_stream,
            ..self
        }
    }
}

options! {
    #[derive(Clone, Default)]
    pub struct GetStateProjectionOptions {
        pub(crate) partition: String,
    }
}

impl GetStateProjectionOptions {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn partition(self, value: impl AsRef<str>) -> Self {
        Self {
            partition: value.as_ref().to_string(),
            ..self
        }
    }
}

options! {
    #[derive(Clone, Default)]
    pub struct GetResultProjectionOptions {
        pub(crate) partition: String,
    }
}

impl GetResultProjectionOptions {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn partition(self, value: impl AsRef<str>) -> Self {
        Self {
            partition: value.as_ref().to_string(),
            ..self
        }
    }
}

options! {
    #[derive(Clone, Default)]
    pub struct GenericProjectionOptions {}
}
