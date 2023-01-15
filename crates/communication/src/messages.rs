use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

pub type CyclerInstance = String;
pub type Path = String;
pub type Reason = String;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Request {
    Injections(InjectionRequest),
    Outputs(OutputRequest),
    Parameters(ParameterRequest),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Response {
    Textual(TextualResponse),
    Binary(BinaryResponse),
    Close { code: CloseCode, reason: Reason },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TextualResponse {
    Injections(InjectionResponse),
    Outputs(TextualOutputResponse),
    Parameters(ParameterResponse),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BinaryResponse {
    Outputs(BinaryOutputResponse),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum InjectionRequest {
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
pub enum InjectionResponse {
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
pub enum OutputRequest {
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
pub enum TextualOutputResponse {
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
pub enum BinaryOutputResponse {
    GetNext {
        reference_id: usize,
        data: Vec<u8>,
    },
    SubscribedData {
        referenced_items: HashMap<usize, Vec<u8>>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ParameterRequest {
    GetFields { id: usize },
    GetCurrent { id: usize, path: Path },
    Subscribe { id: usize, path: Path },
    Unsubscribe { id: usize, path: Path },
    Update { id: usize, path: Path, data: Value },
    UnsubscribeEverything,
    LoadFromDisk { id: usize },
    StoreToDisk { id: usize },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ParameterResponse {
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
        // TODO: maybe also in which ID to store?
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Format {
    Textual,
    Binary,
}
