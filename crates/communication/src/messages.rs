use std::{collections::BTreeMap, time::SystemTime};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Entry {
    pub is_readable: bool,
    pub is_writable: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum TextOrBinary {
    Text(Value),
    Binary(Vec<u8>),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Hash)]
#[non_exhaustive]
pub enum Format {
    Text,
    Binary,
}

pub type Path = String;
pub type Error = String;
pub type RequestId = usize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Request {
    pub id: RequestId,
    pub kind: RequestKind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum RequestKind {
    GetPaths,
    Read { path: Path, format: Format },
    Subscribe { path: Path, format: Format },
    Unsubscribe { id: RequestId },
    Write { path: Path, value: TextOrBinary },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Response {
    pub id: RequestId,
    pub kind: Result<ResponseKind, Error>,
}

pub type Paths = BTreeMap<Path, Entry>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub enum ResponseKind {
    Paths {
        paths: Paths,
    },
    Read {
        timestamp: SystemTime,
        value: TextOrBinary,
    },
    Subscribe {
        timestamp: SystemTime,
        value: TextOrBinary,
    },
    Update {
        timestamp: SystemTime,
        value: TextOrBinary,
    },
    Unsubscribe,
    Write,
}
