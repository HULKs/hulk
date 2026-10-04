//! Reports use a CDR string containing their formatted error chain. Decoding creates a
//! text-only report; concrete source types and captured backtraces do not cross the wire.
//! Serde fields containing `Report` or `Arc<Report>` can use
//! `#[serde(with = "ros_z::message::report")]` with the same representation.

use color_eyre::Report;
use ros_z_schema::{SchemaError, TypeDef};
use serde::{Deserialize, Deserializer, Serializer};
use zenoh::shm::{PosixShmProviderBackend, ShmProvider};
use zenoh_buffers::ZBuf;

use super::{
    CDR_HEADER_LE, CdrEncodeError, CdrError, Message, SerdeCdrCodec, WireDecoder, WireEncoder,
};
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

impl Message for Report {
    type Codec = ReportCodec;

    fn type_name() -> String {
        "color_eyre::Report".to_owned()
    }
}

impl MessageSchema for Report {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        String::build_schema(builder)
    }
}

pub struct ReportCodec;

impl WireEncoder for ReportCodec {
    type Input<'a> = &'a Report;
    type Error = CdrEncodeError;

    fn serialize_to_zbuf(input: &Report) -> Result<ZBuf, Self::Error> {
        SerdeCdrCodec::<String>::serialize_to_zbuf(&format!("{input:#}"))
    }

    fn serialize_to_zbuf_with_hint(
        input: &Report,
        capacity_hint: usize,
    ) -> Result<ZBuf, Self::Error> {
        SerdeCdrCodec::<String>::serialize_to_zbuf_with_hint(&format!("{input:#}"), capacity_hint)
    }

    fn serialized_size_hint(input: &Report) -> usize {
        // CDR header, string length, UTF-8 payload, and terminating NUL.
        CDR_HEADER_LE.len() + size_of::<u32>() + format!("{input:#}").len() + 1
    }

    fn serialize_to_shm(
        input: &Report,
        estimated_size: usize,
        provider: &ShmProvider<PosixShmProviderBackend>,
    ) -> crate::Result<(ZBuf, usize)> {
        SerdeCdrCodec::<String>::serialize_to_shm(&format!("{input:#}"), estimated_size, provider)
    }

    fn serialize_to_buf(input: &Report, buffer: &mut Vec<u8>) -> Result<(), Self::Error> {
        SerdeCdrCodec::<String>::serialize_to_buf(&format!("{input:#}"), buffer)
    }
}

impl WireDecoder for ReportCodec {
    type Input<'a> = &'a [u8];
    type Output = Report;
    type Error = CdrError;

    fn deserialize(input: &[u8]) -> Result<Report, Self::Error> {
        SerdeCdrCodec::<String>::deserialize(input).map(Report::msg)
    }
}
