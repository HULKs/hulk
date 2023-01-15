use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

pub type CyclerInstance = String;
pub type Path = String;
pub type Reason = String;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Request {
    Injections(InjectionsRequest),
    Outputs(OutputsRequest),
    Parameters(ParametersRequest),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Response {
    Textual(TextualResponse),
    Binary(BinaryResponse),
    Close { code: CloseCode, reason: Reason },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TextualResponse {
    Injections(InjectionsResponse),
    Outputs(TextualOutputsResponse),
    Parameters(ParametersResponse),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BinaryResponse {
    Outputs(BinaryOutputsResponse),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum InjectionsRequest {
    Set {
        id: usize,
        cycler_instance: CyclerInstance,
        path: Path,
        data: Value,
    },
    Unset {
        id: usize,
        cycler_instance: CyclerInstance,
        path: Path,
    },
    UnsetEverything,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum InjectionsResponse {
    Set {
        id: usize,
        result: Result<(), Reason>,
    },
    Unset {
        id: usize,
        result: Result<(), Reason>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OutputsRequest {
    GetFields {
        id: usize,
    },
    GetNext {
        id: usize,
        cycler_instance: CyclerInstance,
        path: Path,
        format: Format,
    },
    Subscribe {
        id: usize,
        cycler_instance: CyclerInstance,
        path: Path,
        format: Format,
    },
    Unsubscribe {
        id: usize,
        subscription_id: usize,
    },
    UnsubscribeEverything,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TextualOutputsResponse {
    GetFields {
        id: usize,
        fields: BTreeMap<CyclerInstance, BTreeSet<Path>>,
    },
    GetNext {
        id: usize,
        result: Result<TextualDataOrBinaryReference, Reason>,
    },
    Subscribe {
        id: usize,
        result: Result<(), Reason>,
    },
    Unsubscribe {
        id: usize,
        result: Result<(), Reason>,
    },
    SubscribedData {
        items: HashMap<usize, TextualDataOrBinaryReference>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TextualDataOrBinaryReference {
    TextualData { data: Value },
    BinaryReference { reference_id: usize },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BinaryOutputsResponse {
    GetNext {
        reference_id: usize,
        data: Vec<u8>,
    },
    SubscribedData {
        referenced_items: HashMap<usize, Vec<u8>>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ParametersRequest {
    GetFields { id: usize },
    GetCurrent { id: usize, path: Path },
    Subscribe { id: usize, path: Path },
    Unsubscribe { id: usize, subscription_id: usize },
    UnsubscribeEverything,
    Update { id: usize, path: Path, data: Value },
    LoadFromDisk { id: usize },
    StoreToDisk { id: usize },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ParametersResponse {
    GetFields {
        id: usize,
        fields: BTreeSet<Path>,
    },
    GetCurrent {
        id: usize,
        result: Result<Value, Reason>,
    },
    Subscribe {
        id: usize,
        result: Result<(), Reason>,
    },
    Unsubscribe {
        id: usize,
        result: Result<(), Reason>,
    },
    SubscribedData {
        subscription_id: usize,
        data: Value,
    },
    Update {
        id: usize,
        result: Result<(), Reason>,
    },
    LoadFromDisk {
        id: usize,
        result: Result<(), Reason>,
    },
    StoreToDisk {
        id: usize,
        result: Result<(), Reason>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Format {
    Textual,
    Binary,
}
