use std::{
    fmt,
    ops::Sub,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TwixTime(Duration);

impl TwixTime {
    pub fn from_duration(duration: Duration) -> Self {
        Self(duration)
    }

    pub fn from_system_time(time: SystemTime) -> Option<Self> {
        time.duration_since(UNIX_EPOCH).ok().map(Self)
    }

    pub fn from_nanos(nanos: i64) -> Self {
        Self(Duration::from_nanos(nanos.max(0) as u64))
    }

    pub fn as_system_time(self) -> SystemTime {
        UNIX_EPOCH + self.0
    }

    pub fn as_nanos(self) -> i64 {
        self.0.as_nanos().min(i64::MAX as u128) as i64
    }

    pub fn saturating_duration_since(self, earlier: Self) -> Duration {
        self.0.saturating_sub(earlier.0)
    }

    pub fn duration_since(self, earlier: Self) -> Result<Duration, Duration> {
        if self.0 >= earlier.0 {
            Ok(self.0 - earlier.0)
        } else {
            Err(earlier.0 - self.0)
        }
    }

    pub fn checked_sub(self, duration: Duration) -> Option<Self> {
        self.0.checked_sub(duration).map(Self)
    }
}

impl Sub<Duration> for TwixTime {
    type Output = TwixTime;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self(self.0.saturating_sub(rhs))
    }
}

impl fmt::Display for TwixTime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}.{:09}",
            self.0.as_secs(),
            self.0.subsec_nanos()
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopicDescriptor {
    pub name: String,
    pub graph_type: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TopicListState {
    pub discovering: bool,
    pub topics: Vec<TopicDescriptor>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigNodeDescriptor {
    pub node_fqn: String,
    pub metadata_capable: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfigNodeListState {
    pub discovering: bool,
    pub nodes: Vec<ConfigNodeDescriptor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BackendCapability {
    TopicDiscovery,
    DynamicInspection,
    TypedSubscription,
    NodeConfigRead,
    NodeConfigMetadata,
    NodeConfigWrite,
    ValueWrite,
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("operation `{operation}` is unsupported by the active backend")]
    UnsupportedCapability { operation: &'static str },
    #[error("backend is not connected")]
    NotConnected,
    #[error("logical path is not mapped in the current backend: {path}")]
    UnmappedLogicalPath { path: String },
    #[error("{operation} failed: {message}")]
    Operation {
        operation: &'static str,
        message: String,
    },
}

pub type BackendResult<T> = Result<T, BackendError>;
