//! Serde helpers that encode a `Report` as a string containing its formatted error chain.
//! Decoding creates a text-only report; concrete source types and captured backtraces do not
//! cross the wire. Use with `#[serde(with = "ros_z::message::report")]` on `Report` or
//! `Arc<Report>` fields.

use color_eyre::Report;
use ros_z_schema::{SchemaError, TypeDef};
use serde::{Deserialize, Deserializer, Serializer};

use crate::schema::{MessageSchema, SchemaBuilder};

pub fn serialize<S>(report: &Report, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_str(&format_args!("{report:#}"))
}

pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: From<Report>,
{
    String::deserialize(deserializer).map(|message| Report::msg(message).into())
}

impl MessageSchema for Report {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        String::build_schema(builder)
    }
}
