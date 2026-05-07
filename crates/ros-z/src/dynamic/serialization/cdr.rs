//! CDR serialization for dynamic messages.
//!
//! This module uses the low-level primitives from `ros-z-cdr` for CDR
//! serialization and deserialization of dynamic messages.

use ros_z_cdr::{CdrReader, CdrWriter, LittleEndian};
use zenoh_buffers::ZBuf;

use crate::dynamic::error::DynamicError;
use crate::dynamic::message::DynamicStruct;
use crate::dynamic::schema::Schema;
use crate::dynamic::value::{DynamicNamedValue, DynamicValue, EnumPayloadValue, EnumValue};

use super::CDR_HEADER_LE;
use ros_z_schema::{
    EnumPayloadDef, FieldDef, PrimitiveTypeDef, SequenceLengthDef, TypeDef, TypeDefinition,
};

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
    schema
        .validate()
        .map_err(|error| DynamicError::DeserializationError(error.to_string()))?;
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
    match deserialize_shape_value(&schema.root, schema, &mut reader)? {
        DynamicValue::Struct(value) => Ok(*value),
        _ => Err(DynamicError::DeserializationError(
            "root schema did not deserialize to a struct".into(),
        )),
    }
}

/// Deserialize a dynamic root value from CDR bytes.
pub fn deserialize_cdr_value(data: &[u8], schema: &Schema) -> Result<DynamicValue, DynamicError> {
    schema
        .validate()
        .map_err(|error| DynamicError::DeserializationError(error.to_string()))?;
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
    serialize_shape_value(value, &schema.root, schema, writer)
}

fn serialize_struct_fields(
    value: &DynamicStruct,
    fields: &[FieldDef],
    schema: &Schema,
    writer: &mut CdrWriter<LittleEndian>,
) -> Result<(), DynamicError> {
    value.validate_fields(fields, schema)?;
    for (field, field_value) in fields.iter().zip(value.values().iter()) {
        serialize_shape_value(field_value, &field.shape, schema, writer)?;
    }
    Ok(())
}

fn serialize_shape_value(
    value: &DynamicValue,
    shape: &TypeDef,
    schema: &Schema,
    writer: &mut CdrWriter<LittleEndian>,
) -> Result<(), DynamicError> {
    match (value, shape) {
        (DynamicValue::Bool(v), TypeDef::Primitive(PrimitiveTypeDef::Bool)) => {
            writer.write_bool(*v)
        }
        (DynamicValue::Int8(v), TypeDef::Primitive(PrimitiveTypeDef::I8)) => writer.write_i8(*v),
        (DynamicValue::Uint8(v), TypeDef::Primitive(PrimitiveTypeDef::U8)) => writer.write_u8(*v),
        (DynamicValue::Int16(v), TypeDef::Primitive(PrimitiveTypeDef::I16)) => writer.write_i16(*v),
        (DynamicValue::Uint16(v), TypeDef::Primitive(PrimitiveTypeDef::U16)) => {
            writer.write_u16(*v)
        }
        (DynamicValue::Int32(v), TypeDef::Primitive(PrimitiveTypeDef::I32)) => writer.write_i32(*v),
        (DynamicValue::Uint32(v), TypeDef::Primitive(PrimitiveTypeDef::U32)) => {
            writer.write_u32(*v)
        }
        (DynamicValue::Int64(v), TypeDef::Primitive(PrimitiveTypeDef::I64)) => writer.write_i64(*v),
        (DynamicValue::Uint64(v), TypeDef::Primitive(PrimitiveTypeDef::U64)) => {
            writer.write_u64(*v)
        }
        (DynamicValue::Float32(v), TypeDef::Primitive(PrimitiveTypeDef::F32)) => {
            writer.write_f32(*v)
        }
        (DynamicValue::Float64(v), TypeDef::Primitive(PrimitiveTypeDef::F64)) => {
            writer.write_f64(*v)
        }
        (DynamicValue::String(v), TypeDef::String) => writer.write_string(v),
        (DynamicValue::Optional(None), TypeDef::Optional(_)) => writer.write_u32(0),
        (DynamicValue::Optional(Some(value)), TypeDef::Optional(element)) => {
            writer.write_u32(1);
            serialize_shape_value(value, element, schema, writer)?;
        }
        (DynamicValue::Sequence(values), TypeDef::Sequence { element, length }) => {
            if matches!(length, SequenceLengthDef::Dynamic) {
                writer.write_sequence_length(values.len());
            }
            for value in values {
                serialize_shape_value(value, element, schema, writer)?;
            }
        }
        (DynamicValue::Bytes(bytes), TypeDef::Sequence { element, length })
            if matches!(element.as_ref(), TypeDef::Primitive(PrimitiveTypeDef::U8)) =>
        {
            match length {
                SequenceLengthDef::Dynamic => writer.write_bytes(bytes),
                SequenceLengthDef::Fixed(_) => {
                    for byte in bytes {
                        writer.write_u8(*byte);
                    }
                }
            }
        }
        (DynamicValue::Struct(value), TypeDef::Named(name)) => match schema.definitions.get(name) {
            Some(TypeDefinition::Struct(definition)) => {
                serialize_struct_fields(value, &definition.fields, schema, writer)?
            }
            _ => {
                return Err(DynamicError::SerializationError(format!(
                    "named struct definition {name} not found"
                )));
            }
        },
        (DynamicValue::Map(entries), TypeDef::Map { key, value }) => {
            writer.write_sequence_length(entries.len());
            for (entry_key, entry_value) in entries {
                serialize_shape_value(entry_key, key, schema, writer)?;
                serialize_shape_value(entry_value, value, schema, writer)?;
            }
        }
        (DynamicValue::Enum(enum_value), TypeDef::Named(name)) => {
            match schema.definitions.get(name) {
                Some(TypeDefinition::Enum(definition)) => {
                    let variant = definition
                        .variants
                        .get(enum_value.variant_index as usize)
                        .ok_or_else(|| {
                            DynamicError::SerializationError(format!(
                                "enum variant index {} is out of bounds",
                                enum_value.variant_index
                            ))
                        })?;
                    writer.write_u32(enum_value.variant_index);
                    serialize_runtime_enum_payload(
                        &enum_value.payload,
                        &variant.payload,
                        schema,
                        writer,
                    )?;
                }
                _ => {
                    return Err(DynamicError::SerializationError(format!(
                        "named enum definition {name} not found"
                    )));
                }
            }
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
    deserialize_shape_value(&schema.root, schema, reader)
}

fn deserialize_shape_value(
    shape: &TypeDef,
    schema: &Schema,
    reader: &mut CdrReader<LittleEndian>,
) -> Result<DynamicValue, DynamicError> {
    match shape {
        TypeDef::Primitive(PrimitiveTypeDef::Bool) => {
            Ok(DynamicValue::Bool(reader.read_bool().map_err(map_cdr_err)?))
        }
        TypeDef::Primitive(PrimitiveTypeDef::I8) => {
            Ok(DynamicValue::Int8(reader.read_i8().map_err(map_cdr_err)?))
        }
        TypeDef::Primitive(PrimitiveTypeDef::U8) => {
            Ok(DynamicValue::Uint8(reader.read_u8().map_err(map_cdr_err)?))
        }
        TypeDef::Primitive(PrimitiveTypeDef::I16) => {
            Ok(DynamicValue::Int16(reader.read_i16().map_err(map_cdr_err)?))
        }
        TypeDef::Primitive(PrimitiveTypeDef::U16) => Ok(DynamicValue::Uint16(
            reader.read_u16().map_err(map_cdr_err)?,
        )),
        TypeDef::Primitive(PrimitiveTypeDef::I32) => {
            Ok(DynamicValue::Int32(reader.read_i32().map_err(map_cdr_err)?))
        }
        TypeDef::Primitive(PrimitiveTypeDef::U32) => Ok(DynamicValue::Uint32(
            reader.read_u32().map_err(map_cdr_err)?,
        )),
        TypeDef::Primitive(PrimitiveTypeDef::I64) => {
            Ok(DynamicValue::Int64(reader.read_i64().map_err(map_cdr_err)?))
        }
        TypeDef::Primitive(PrimitiveTypeDef::U64) => Ok(DynamicValue::Uint64(
            reader.read_u64().map_err(map_cdr_err)?,
        )),
        TypeDef::Primitive(PrimitiveTypeDef::F32) => Ok(DynamicValue::Float32(
            reader.read_f32().map_err(map_cdr_err)?,
        )),
        TypeDef::Primitive(PrimitiveTypeDef::F64) => Ok(DynamicValue::Float64(
            reader.read_f64().map_err(map_cdr_err)?,
        )),
        TypeDef::String => Ok(DynamicValue::String(
            reader.read_string().map_err(map_cdr_err)?,
        )),
        TypeDef::Optional(element) => match reader.read_u32().map_err(map_cdr_err)? {
            0 => Ok(DynamicValue::Optional(None)),
            1 => Ok(DynamicValue::Optional(Some(Box::new(
                deserialize_shape_value(element, schema, reader)?,
            )))),
            other => Err(DynamicError::DeserializationError(format!(
                "invalid option discriminant: {other}"
            ))),
        },
        TypeDef::Sequence { element, length } => match length {
            SequenceLengthDef::Dynamic
                if matches!(element.as_ref(), TypeDef::Primitive(PrimitiveTypeDef::U8)) =>
            {
                Ok(DynamicValue::Bytes(
                    reader.read_byte_sequence().map_err(map_cdr_err)?.to_vec(),
                ))
            }
            SequenceLengthDef::Dynamic => {
                let len = reader.read_sequence_length().map_err(map_cdr_err)?;
                let mut values = Vec::with_capacity(len);
                for _ in 0..len {
                    values.push(deserialize_shape_value(element, schema, reader)?);
                }
                Ok(DynamicValue::Sequence(values))
            }
            SequenceLengthDef::Fixed(len) => {
                let mut values = Vec::with_capacity(*len);
                for _ in 0..*len {
                    values.push(deserialize_shape_value(element, schema, reader)?);
                }
                Ok(DynamicValue::Sequence(values))
            }
        },
        TypeDef::Named(name) => match schema.definitions.get(name) {
            Some(TypeDefinition::Struct(definition)) => {
                let mut values = Vec::with_capacity(definition.fields.len());
                for field in &definition.fields {
                    values.push(deserialize_shape_value(&field.shape, schema, reader)?);
                }
                Ok(DynamicValue::Struct(Box::new(
                    DynamicStruct::from_values_unchecked(
                        std::sync::Arc::clone(schema),
                        name.clone(),
                        values,
                    ),
                )))
            }
            Some(TypeDefinition::Enum(definition)) => {
                let variant_index = reader.read_u32().map_err(map_cdr_err)?;
                let variant = definition
                    .variants
                    .get(variant_index as usize)
                    .ok_or_else(|| {
                        DynamicError::DeserializationError(format!(
                            "enum variant index {} is out of bounds",
                            variant_index
                        ))
                    })?;
                Ok(DynamicValue::Enum(EnumValue {
                    variant_index,
                    variant_name: variant.name.clone(),
                    payload: deserialize_runtime_enum_payload(&variant.payload, schema, reader)?,
                }))
            }
            None => Err(DynamicError::DeserializationError(format!(
                "named definition {name} not found"
            ))),
        },
        TypeDef::Map { key, value } => {
            let len = reader.read_sequence_length().map_err(map_cdr_err)?;
            let mut entries = Vec::with_capacity(len);
            for _ in 0..len {
                entries.push((
                    deserialize_shape_value(key, schema, reader)?,
                    deserialize_shape_value(value, schema, reader)?,
                ));
            }
            Ok(DynamicValue::Map(entries))
        }
    }
}

fn serialize_runtime_enum_payload(
    payload: &EnumPayloadValue,
    schema: &EnumPayloadDef,
    bundle: &Schema,
    writer: &mut CdrWriter<LittleEndian>,
) -> Result<(), DynamicError> {
    match (payload, schema) {
        (EnumPayloadValue::Unit, EnumPayloadDef::Unit) => Ok(()),
        (EnumPayloadValue::Newtype(value), EnumPayloadDef::Newtype(schema)) => {
            serialize_shape_value(value, schema, bundle, writer)
        }
        (EnumPayloadValue::Tuple(values), EnumPayloadDef::Tuple(schemas)) => {
            for (value, schema) in values.iter().zip(schemas.iter()) {
                serialize_shape_value(value, schema, bundle, writer)?;
            }
            Ok(())
        }
        (EnumPayloadValue::Struct(values), EnumPayloadDef::Struct(fields)) => {
            for (value, field) in values.iter().zip(fields.iter()) {
                serialize_shape_value(&value.value, &field.shape, bundle, writer)?;
            }
            Ok(())
        }
        _ => Err(DynamicError::SerializationError(
            "enum payload mismatch".into(),
        )),
    }
}

fn deserialize_runtime_enum_payload(
    schema: &EnumPayloadDef,
    bundle: &Schema,
    reader: &mut CdrReader<LittleEndian>,
) -> Result<EnumPayloadValue, DynamicError> {
    match schema {
        EnumPayloadDef::Unit => Ok(EnumPayloadValue::Unit),
        EnumPayloadDef::Newtype(schema) => Ok(EnumPayloadValue::Newtype(Box::new(
            deserialize_shape_value(schema, bundle, reader)?,
        ))),
        EnumPayloadDef::Tuple(schemas) => schemas
            .iter()
            .map(|schema| deserialize_shape_value(schema, bundle, reader))
            .collect::<Result<Vec<_>, _>>()
            .map(EnumPayloadValue::Tuple),
        EnumPayloadDef::Struct(fields) => fields
            .iter()
            .map(|field| {
                Ok(DynamicNamedValue {
                    name: field.name.clone(),
                    value: deserialize_shape_value(&field.shape, bundle, reader)?,
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
