//! CDR serialization for dynamic messages.
//!
//! This module uses the low-level primitives from `ros-z-cdr` for CDR
//! serialization and deserialization of dynamic messages.

use ros_z_cdr::{CdrReader, CdrWriter, LittleEndian};
use zenoh_buffers::ZBuf;

use crate::dynamic::error::DynamicError;
use crate::dynamic::message::DynamicStruct;
use crate::dynamic::schema::{
    PrimitiveType, RuntimeDynamicEnumPayload, Schema, SequenceLength, TypeShape,
};
use crate::dynamic::value::{DynamicNamedValue, DynamicValue, EnumPayloadValue, EnumValue};

use super::CDR_HEADER_LE;

/// Serialize a dynamic message to CDR bytes.
pub fn serialize_cdr(message: &DynamicStruct) -> Result<Vec<u8>, DynamicError> {
    let schema = message.schema_arc();
    serialize_cdr_value(&schema, &DynamicValue::Struct(Box::new(message.clone())))
}

/// Serialize a dynamic root value to CDR bytes.
pub fn serialize_cdr_value(schema: &Schema, value: &DynamicValue) -> Result<Vec<u8>, DynamicError> {
    value.validate_against(schema)?;
    let mut buffer = Vec::with_capacity(256);
    buffer.extend_from_slice(&CDR_HEADER_LE);

    let mut writer = CdrWriter::<LittleEndian>::new(&mut buffer);
    serialize_root_value(value, schema, &mut writer)?;

    Ok(buffer)
}

/// Serialize a dynamic message to a ZBuf.
pub fn serialize_cdr_to_zbuf(message: &DynamicStruct) -> Result<ZBuf, DynamicError> {
    let bytes = serialize_cdr(message)?;
    Ok(ZBuf::from(bytes))
}

/// Deserialize a dynamic message from CDR bytes.
pub fn deserialize_cdr(data: &[u8], schema: &Schema) -> Result<DynamicStruct, DynamicError> {
    if data.len() < 4 {
        return Err(DynamicError::DeserializationError(
            "CDR data too short for header".into(),
        ));
    }
    let header = &data[0..4];
    let representation_identifier = &header[0..2];
    if representation_identifier != [0x00, 0x01] {
        return Err(DynamicError::DeserializationError(format!(
            "Expected CDR_LE encapsulation ({:?}), found {:?}",
            [0x00, 0x01],
            representation_identifier
        )));
    }

    let payload = &data[4..];
    let mut reader = CdrReader::<LittleEndian>::new(payload);
    match deserialize_shape_value(schema, &mut reader)? {
        DynamicValue::Struct(value) => Ok(*value),
        _ => Err(DynamicError::DeserializationError(
            "root schema did not deserialize to a struct".into(),
        )),
    }
}

/// Deserialize a dynamic root value from CDR bytes.
pub fn deserialize_cdr_value(data: &[u8], schema: &Schema) -> Result<DynamicValue, DynamicError> {
    if data.len() < 4 {
        return Err(DynamicError::DeserializationError(
            "CDR data too short for header".into(),
        ));
    }
    let header = &data[0..4];
    let representation_identifier = &header[0..2];
    if representation_identifier != [0x00, 0x01] {
        return Err(DynamicError::DeserializationError(format!(
            "Expected CDR_LE encapsulation ({:?}), found {:?}",
            [0x00, 0x01],
            representation_identifier
        )));
    }

    let payload = &data[4..];
    let mut reader = CdrReader::<LittleEndian>::new(payload);
    deserialize_root_value(schema, &mut reader)
}

fn serialize_root_value(
    value: &DynamicValue,
    schema: &Schema,
    writer: &mut CdrWriter<LittleEndian>,
) -> Result<(), DynamicError> {
    match (value, schema.as_ref()) {
        (DynamicValue::Struct(value), TypeShape::Struct { fields, .. }) => {
            serialize_struct_fields(value, fields, writer)
        }
        _ => serialize_shape_value(value, schema, writer),
    }
}

fn serialize_struct_fields(
    value: &DynamicStruct,
    fields: &[crate::dynamic::schema::FieldSchema],
    writer: &mut CdrWriter<LittleEndian>,
) -> Result<(), DynamicError> {
    value.validate_fields(fields)?;
    for (field, field_value) in fields.iter().zip(value.values().iter()) {
        serialize_shape_value(field_value, &field.schema, writer)?;
    }
    Ok(())
}

fn serialize_shape_value(
    value: &DynamicValue,
    schema: &Schema,
    writer: &mut CdrWriter<LittleEndian>,
) -> Result<(), DynamicError> {
    match (value, schema.as_ref()) {
        (DynamicValue::Bool(v), TypeShape::Primitive(PrimitiveType::Bool)) => writer.write_bool(*v),
        (DynamicValue::Int8(v), TypeShape::Primitive(PrimitiveType::I8)) => writer.write_i8(*v),
        (DynamicValue::Uint8(v), TypeShape::Primitive(PrimitiveType::U8)) => writer.write_u8(*v),
        (DynamicValue::Int16(v), TypeShape::Primitive(PrimitiveType::I16)) => writer.write_i16(*v),
        (DynamicValue::Uint16(v), TypeShape::Primitive(PrimitiveType::U16)) => writer.write_u16(*v),
        (DynamicValue::Int32(v), TypeShape::Primitive(PrimitiveType::I32)) => writer.write_i32(*v),
        (DynamicValue::Uint32(v), TypeShape::Primitive(PrimitiveType::U32)) => writer.write_u32(*v),
        (DynamicValue::Int64(v), TypeShape::Primitive(PrimitiveType::I64)) => writer.write_i64(*v),
        (DynamicValue::Uint64(v), TypeShape::Primitive(PrimitiveType::U64)) => writer.write_u64(*v),
        (DynamicValue::Float32(v), TypeShape::Primitive(PrimitiveType::F32)) => {
            writer.write_f32(*v)
        }
        (DynamicValue::Float64(v), TypeShape::Primitive(PrimitiveType::F64)) => {
            writer.write_f64(*v)
        }
        (DynamicValue::String(v), TypeShape::String) => writer.write_string(v),
        (DynamicValue::Optional(None), TypeShape::Optional(_)) => writer.write_u32(0),
        (DynamicValue::Optional(Some(value)), TypeShape::Optional(schema)) => {
            writer.write_u32(1);
            serialize_shape_value(value, schema, writer)?;
        }
        (DynamicValue::Sequence(values), TypeShape::Sequence { element, length }) => {
            if matches!(length, SequenceLength::Dynamic) {
                writer.write_sequence_length(values.len());
            }
            for value in values {
                serialize_shape_value(value, element, writer)?;
            }
        }
        (DynamicValue::Bytes(bytes), TypeShape::Sequence { element, length })
            if matches!(element.as_ref(), TypeShape::Primitive(PrimitiveType::U8)) =>
        {
            match length {
                SequenceLength::Dynamic => writer.write_bytes(bytes),
                SequenceLength::Fixed(_) => {
                    for byte in bytes {
                        writer.write_u8(*byte);
                    }
                }
            }
        }
        (DynamicValue::Struct(value), TypeShape::Struct { fields, .. }) => {
            serialize_struct_fields(value, fields, writer)?
        }
        (DynamicValue::Map(entries), TypeShape::Map { key, value }) => {
            writer.write_sequence_length(entries.len());
            for (entry_key, entry_value) in entries {
                serialize_shape_value(entry_key, key, writer)?;
                serialize_shape_value(entry_value, value, writer)?;
            }
        }
        (DynamicValue::Enum(enum_value), TypeShape::Enum { variants, .. }) => {
            let variant = variants
                .get(enum_value.variant_index as usize)
                .ok_or_else(|| {
                    DynamicError::SerializationError(format!(
                        "enum variant index {} is out of bounds",
                        enum_value.variant_index
                    ))
                })?;
            writer.write_u32(enum_value.variant_index);
            serialize_runtime_enum_payload(&enum_value.payload, &variant.payload, writer)?;
        }
        _ => {
            return Err(DynamicError::SerializationError(format!(
                "Type mismatch: cannot serialize {:?} as {:?}",
                value, schema
            )));
        }
    }
    Ok(())
}

fn deserialize_root_value(
    schema: &Schema,
    reader: &mut CdrReader<LittleEndian>,
) -> Result<DynamicValue, DynamicError> {
    deserialize_shape_value(schema, reader)
}

fn deserialize_shape_value(
    schema: &Schema,
    reader: &mut CdrReader<LittleEndian>,
) -> Result<DynamicValue, DynamicError> {
    match schema.as_ref() {
        TypeShape::Primitive(PrimitiveType::Bool) => {
            Ok(DynamicValue::Bool(reader.read_bool().map_err(map_cdr_err)?))
        }
        TypeShape::Primitive(PrimitiveType::I8) => {
            Ok(DynamicValue::Int8(reader.read_i8().map_err(map_cdr_err)?))
        }
        TypeShape::Primitive(PrimitiveType::U8) => {
            Ok(DynamicValue::Uint8(reader.read_u8().map_err(map_cdr_err)?))
        }
        TypeShape::Primitive(PrimitiveType::I16) => {
            Ok(DynamicValue::Int16(reader.read_i16().map_err(map_cdr_err)?))
        }
        TypeShape::Primitive(PrimitiveType::U16) => Ok(DynamicValue::Uint16(
            reader.read_u16().map_err(map_cdr_err)?,
        )),
        TypeShape::Primitive(PrimitiveType::I32) => {
            Ok(DynamicValue::Int32(reader.read_i32().map_err(map_cdr_err)?))
        }
        TypeShape::Primitive(PrimitiveType::U32) => Ok(DynamicValue::Uint32(
            reader.read_u32().map_err(map_cdr_err)?,
        )),
        TypeShape::Primitive(PrimitiveType::I64) => {
            Ok(DynamicValue::Int64(reader.read_i64().map_err(map_cdr_err)?))
        }
        TypeShape::Primitive(PrimitiveType::U64) => Ok(DynamicValue::Uint64(
            reader.read_u64().map_err(map_cdr_err)?,
        )),
        TypeShape::Primitive(PrimitiveType::F32) => Ok(DynamicValue::Float32(
            reader.read_f32().map_err(map_cdr_err)?,
        )),
        TypeShape::Primitive(PrimitiveType::F64) => Ok(DynamicValue::Float64(
            reader.read_f64().map_err(map_cdr_err)?,
        )),
        TypeShape::String => Ok(DynamicValue::String(
            reader.read_string().map_err(map_cdr_err)?,
        )),
        TypeShape::Optional(schema) => match reader.read_u32().map_err(map_cdr_err)? {
            0 => Ok(DynamicValue::Optional(None)),
            1 => Ok(DynamicValue::Optional(Some(Box::new(
                deserialize_shape_value(schema, reader)?,
            )))),
            other => Err(DynamicError::DeserializationError(format!(
                "invalid option discriminant: {other}"
            ))),
        },
        TypeShape::Sequence { element, length } => match length {
            SequenceLength::Dynamic
                if matches!(element.as_ref(), TypeShape::Primitive(PrimitiveType::U8)) =>
            {
                Ok(DynamicValue::Bytes(
                    reader.read_byte_sequence().map_err(map_cdr_err)?.to_vec(),
                ))
            }
            SequenceLength::Dynamic => {
                let len = reader.read_sequence_length().map_err(map_cdr_err)?;
                let mut values = Vec::with_capacity(len);
                for _ in 0..len {
                    values.push(deserialize_shape_value(element, reader)?);
                }
                Ok(DynamicValue::Sequence(values))
            }
            SequenceLength::Fixed(len) => {
                let mut values = Vec::with_capacity(*len);
                for _ in 0..*len {
                    values.push(deserialize_shape_value(element, reader)?);
                }
                Ok(DynamicValue::Sequence(values))
            }
        },
        TypeShape::Struct { fields, .. } => {
            let mut values = Vec::with_capacity(fields.len());
            for field in fields {
                values.push(deserialize_shape_value(&field.schema, reader)?);
            }
            Ok(DynamicValue::Struct(Box::new(DynamicStruct::from_values(
                schema, values,
            ))))
        }
        TypeShape::Map { key, value } => {
            let len = reader.read_sequence_length().map_err(map_cdr_err)?;
            let mut entries = Vec::with_capacity(len);
            for _ in 0..len {
                entries.push((
                    deserialize_shape_value(key, reader)?,
                    deserialize_shape_value(value, reader)?,
                ));
            }
            Ok(DynamicValue::Map(entries))
        }
        TypeShape::Enum { variants, .. } => {
            let variant_index = reader.read_u32().map_err(map_cdr_err)?;
            let variant = variants.get(variant_index as usize).ok_or_else(|| {
                DynamicError::DeserializationError(format!(
                    "enum variant index {} is out of bounds",
                    variant_index
                ))
            })?;
            Ok(DynamicValue::Enum(EnumValue {
                variant_index,
                variant_name: variant.name.clone(),
                payload: deserialize_runtime_enum_payload(&variant.payload, reader)?,
            }))
        }
    }
}

fn serialize_runtime_enum_payload(
    payload: &EnumPayloadValue,
    schema: &RuntimeDynamicEnumPayload,
    writer: &mut CdrWriter<LittleEndian>,
) -> Result<(), DynamicError> {
    match (payload, schema) {
        (EnumPayloadValue::Unit, RuntimeDynamicEnumPayload::Unit) => Ok(()),
        (EnumPayloadValue::Newtype(value), RuntimeDynamicEnumPayload::Newtype(schema)) => {
            serialize_shape_value(value, schema, writer)
        }
        (EnumPayloadValue::Tuple(values), RuntimeDynamicEnumPayload::Tuple(schemas)) => {
            for (value, schema) in values.iter().zip(schemas.iter()) {
                serialize_shape_value(value, schema, writer)?;
            }
            Ok(())
        }
        (EnumPayloadValue::Struct(values), RuntimeDynamicEnumPayload::Struct(fields)) => {
            for (value, field) in values.iter().zip(fields.iter()) {
                serialize_shape_value(&value.value, &field.schema, writer)?;
            }
            Ok(())
        }
        _ => Err(DynamicError::SerializationError(
            "enum payload mismatch".into(),
        )),
    }
}

fn deserialize_runtime_enum_payload(
    schema: &RuntimeDynamicEnumPayload,
    reader: &mut CdrReader<LittleEndian>,
) -> Result<EnumPayloadValue, DynamicError> {
    match schema {
        RuntimeDynamicEnumPayload::Unit => Ok(EnumPayloadValue::Unit),
        RuntimeDynamicEnumPayload::Newtype(schema) => Ok(EnumPayloadValue::Newtype(Box::new(
            deserialize_shape_value(schema, reader)?,
        ))),
        RuntimeDynamicEnumPayload::Tuple(schemas) => schemas
            .iter()
            .map(|schema| deserialize_shape_value(schema, reader))
            .collect::<Result<Vec<_>, _>>()
            .map(EnumPayloadValue::Tuple),
        RuntimeDynamicEnumPayload::Struct(fields) => fields
            .iter()
            .map(|field| {
                Ok(DynamicNamedValue {
                    name: field.name.clone(),
                    value: deserialize_shape_value(&field.schema, reader)?,
                })
            })
            .collect::<Result<Vec<_>, DynamicError>>()
            .map(EnumPayloadValue::Struct),
    }
}

/// Map ros-z-cdr errors to DynamicError.
fn map_cdr_err(e: ros_z_cdr::Error) -> DynamicError {
    DynamicError::DeserializationError(e.to_string())
}
