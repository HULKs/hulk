//! CDR serialization for dynamic messages.
//!
//! This module uses the low-level primitives from `ros-z-cdr` for CDR
//! serialization and deserialization of dynamic messages.

use ros_z_cdr::{CdrDeserializer, CdrReader, CdrWriter, Error as CdrError, LittleEndian};
use serde::de::DeserializeOwned;
use zenoh_buffers::ZBuf;

use crate::dynamic::error::DynamicError;
use crate::dynamic::message::DynamicStruct;
use crate::dynamic::schema::Schema;
use crate::dynamic::value::{DynamicNamedValue, DynamicValue, EnumPayloadValue, EnumValue};
use crate::dynamic::{CdrSelectionError, SelectedType, SelectionError, ValuePath, ValuePathStep};

use super::CDR_HEADER_LE;
use ros_z_schema::{
    EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, PrimitiveTypeDef, SchemaError,
    SequenceLengthDef, TypeDef, TypeDefinition, TypeName,
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
        .map_err(|error| DynamicError::schema("deserializing dynamic CDR struct", error))?;
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
        .map_err(|error| DynamicError::schema("deserializing dynamic CDR value", error))?;
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
        (DynamicValue::Unit, TypeDef::Primitive(PrimitiveTypeDef::Unit)) => {}
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

/// Seek through the containing message, then decode the leaf at the same alignment origin.
pub(crate) fn deserialize_cdr_selection<T: DeserializeOwned>(
    data: &[u8],
    schema: &Schema,
    path: &ValuePath,
    expected: &TypeDef,
) -> Result<T, CdrSelectionError> {
    let header = data.get(..4).ok_or(CdrError::UnexpectedEof)?;
    let representation = [header[0], header[1]];
    if representation != [0, 1] {
        return Err(CdrSelectionError::Representation(representation));
    }
    let mut reader = CdrReader::<LittleEndian>::new(&data[4..]);
    let selected = seek_selection(schema, path.steps(), &mut reader)?;
    if !matches!(expected, TypeDef::Optional(_)) {
        unwrap_cdr_optional(selected, &mut reader)?;
    }
    Ok(T::deserialize(&mut CdrDeserializer::from_reader(reader))?)
}

fn unwrap_cdr_optional<'a>(
    mut selected: SelectedType<'a>,
    reader: &mut CdrReader<LittleEndian>,
) -> Result<SelectedType<'a>, CdrSelectionError> {
    while let SelectedType::Value(TypeDef::Optional(inner)) = selected {
        match reader.read_u32()? {
            1 => selected = SelectedType::Value(inner),
            0 => return Err(SelectionError::AbsentOptional.into()),
            tag => return Err(CdrError::InvalidOptionTag(tag).into()),
        }
    }
    Ok(selected)
}

fn seek_selection<'a>(
    schema: &'a Schema,
    steps: &[ValuePathStep],
    reader: &mut CdrReader<LittleEndian>,
) -> Result<SelectedType<'a>, CdrSelectionError> {
    let mut selected = SelectedType::Value(&schema.root);
    for step in steps {
        selected = unwrap_cdr_optional(selected, reader)?;
        selected = match step {
            ValuePathStep::Field(name) => {
                SelectedType::Value(seek_field(selected, name, schema, reader)?)
            }
            ValuePathStep::Index(index) => {
                SelectedType::Value(seek_index(selected, *index, schema, reader)?)
            }
            ValuePathStep::Variant(name) => seek_variant(selected, name, schema, reader)?,
        };
    }
    Ok(selected)
}

fn seek_field<'a>(
    selected: SelectedType<'a>,
    name: &str,
    schema: &'a Schema,
    reader: &mut CdrReader<LittleEndian>,
) -> Result<&'a TypeDef, CdrSelectionError> {
    let fields = match selected {
        SelectedType::Value(TypeDef::Named(type_name)) => match definition(schema, type_name)? {
            TypeDefinition::Struct(definition) => &definition.fields,
            _ => return Err(SelectionError::CannotTraverse(name.to_owned()).into()),
        },
        SelectedType::EnumPayload(EnumPayloadDef::Struct(fields)) => fields,
        _ => return Err(SelectionError::CannotTraverse(name.to_owned()).into()),
    };
    for field in fields {
        if field.name == name {
            return Ok(&field.shape);
        }
        skip_value(&field.shape, schema, reader)?;
    }
    Err(SelectionError::Field(name.to_owned()).into())
}

fn seek_index<'a>(
    selected: SelectedType<'a>,
    index: usize,
    schema: &Schema,
    reader: &mut CdrReader<LittleEndian>,
) -> Result<&'a TypeDef, CdrSelectionError> {
    match selected {
        SelectedType::Value(TypeDef::Sequence { element, length }) => {
            let len = sequence_length(length, reader)?;
            if index >= len {
                return Err(SelectionError::AbsentIndex { index, len }.into());
            }
            for _ in 0..index {
                skip_value(element, schema, reader)?;
            }
            Ok(element)
        }
        SelectedType::EnumPayload(EnumPayloadDef::Tuple(fields)) => {
            let shape = fields.get(index).ok_or(SelectionError::InvalidIndex {
                index,
                len: fields.len(),
            })?;
            for field in &fields[..index] {
                skip_value(field, schema, reader)?;
            }
            Ok(shape)
        }
        _ => Err(SelectionError::CannotTraverse(format!("[{index}]")).into()),
    }
}

fn seek_variant<'a>(
    selected: SelectedType<'a>,
    expected: &str,
    schema: &'a Schema,
    reader: &mut CdrReader<LittleEndian>,
) -> Result<SelectedType<'a>, CdrSelectionError> {
    let SelectedType::Value(TypeDef::Named(name)) = selected else {
        return Err(SelectionError::Variant(expected.to_owned()).into());
    };
    let TypeDefinition::Enum(definition) = definition(schema, name)? else {
        return Err(SelectionError::Variant(expected.to_owned()).into());
    };
    let variant = read_variant(definition, reader)?;
    if variant.name != expected {
        return Err(SelectionError::InactiveVariant {
            expected: expected.to_owned(),
            actual: variant.name.clone(),
        }
        .into());
    }
    Ok(match &variant.payload {
        EnumPayloadDef::Newtype(shape) => SelectedType::Value(shape),
        payload => SelectedType::EnumPayload(payload),
    })
}

fn definition<'a>(
    schema: &'a Schema,
    name: &TypeName,
) -> Result<&'a TypeDefinition, CdrSelectionError> {
    schema
        .definitions
        .get(name)
        .ok_or_else(|| SchemaError::MissingDefinition(name.clone()).into())
}

fn read_variant<'a>(
    definition: &'a EnumDef,
    reader: &mut CdrReader<LittleEndian>,
) -> Result<&'a EnumVariantDef, CdrSelectionError> {
    let index = reader.read_u32()?;
    definition
        .variants
        .get(index as usize)
        .ok_or(CdrSelectionError::InvalidEnumTag {
            index,
            len: definition.variants.len(),
        })
}

fn sequence_length(
    length: &SequenceLengthDef,
    reader: &mut CdrReader<LittleEndian>,
) -> Result<usize, CdrSelectionError> {
    Ok(match length {
        SequenceLengthDef::Dynamic => reader.read_sequence_length()?,
        SequenceLengthDef::Fixed(len) => *len,
    })
}

fn skip_value(
    shape: &TypeDef,
    schema: &Schema,
    reader: &mut CdrReader<LittleEndian>,
) -> Result<(), CdrSelectionError> {
    match shape {
        TypeDef::Primitive(primitive) => {
            let width = match primitive {
                PrimitiveTypeDef::Unit => return Ok(()),
                PrimitiveTypeDef::Bool => {
                    reader.read_bool()?;
                    return Ok(());
                }
                PrimitiveTypeDef::I8 | PrimitiveTypeDef::U8 => 1,
                PrimitiveTypeDef::I16 | PrimitiveTypeDef::U16 => 2,
                PrimitiveTypeDef::I32 | PrimitiveTypeDef::U32 | PrimitiveTypeDef::F32 => 4,
                PrimitiveTypeDef::I64 | PrimitiveTypeDef::U64 | PrimitiveTypeDef::F64 => 8,
            };
            reader.align(width)?;
            reader.read_bytes(width)?;
        }
        TypeDef::String => {
            reader.read_str()?;
        }
        TypeDef::Optional(inner) => match reader.read_u32()? {
            0 => {}
            1 => skip_value(inner, schema, reader)?,
            tag => return Err(CdrError::InvalidOptionTag(tag).into()),
        },
        TypeDef::Sequence {
            element,
            length: SequenceLengthDef::Dynamic,
        } if **element == TypeDef::Primitive(PrimitiveTypeDef::U8) => {
            reader.read_byte_sequence()?;
        }
        TypeDef::Sequence { element, length } => {
            for _ in 0..sequence_length(length, reader)? {
                skip_value(element, schema, reader)?;
            }
        }
        TypeDef::Named(name) => match definition(schema, name)? {
            TypeDefinition::Struct(definition) => {
                for field in &definition.fields {
                    skip_value(&field.shape, schema, reader)?;
                }
            }
            TypeDefinition::Enum(definition) => {
                skip_payload(&read_variant(definition, reader)?.payload, schema, reader)?;
            }
        },
        TypeDef::Map { key, value } => {
            for _ in 0..reader.read_sequence_length()? {
                skip_value(key, schema, reader)?;
                skip_value(value, schema, reader)?;
            }
        }
    }
    Ok(())
}

fn skip_payload(
    payload: &EnumPayloadDef,
    schema: &Schema,
    reader: &mut CdrReader<LittleEndian>,
) -> Result<(), CdrSelectionError> {
    match payload {
        EnumPayloadDef::Unit => {}
        EnumPayloadDef::Newtype(shape) => skip_value(shape, schema, reader)?,
        EnumPayloadDef::Tuple(shapes) => {
            for shape in shapes {
                skip_value(shape, schema, reader)?;
            }
        }
        EnumPayloadDef::Struct(fields) => {
            for field in fields {
                skip_value(&field.shape, schema, reader)?;
            }
        }
    }
    Ok(())
}

fn deserialize_shape_value(
    shape: &TypeDef,
    schema: &Schema,
    reader: &mut CdrReader<LittleEndian>,
) -> Result<DynamicValue, DynamicError> {
    match shape {
        TypeDef::Primitive(PrimitiveTypeDef::Unit) => Ok(DynamicValue::Unit),
        TypeDef::Primitive(PrimitiveTypeDef::Bool) => Ok(DynamicValue::Bool(
            reader.read_bool().map_err(DynamicError::deserialization)?,
        )),
        TypeDef::Primitive(PrimitiveTypeDef::I8) => Ok(DynamicValue::Int8(
            reader.read_i8().map_err(DynamicError::deserialization)?,
        )),
        TypeDef::Primitive(PrimitiveTypeDef::U8) => Ok(DynamicValue::Uint8(
            reader.read_u8().map_err(DynamicError::deserialization)?,
        )),
        TypeDef::Primitive(PrimitiveTypeDef::I16) => Ok(DynamicValue::Int16(
            reader.read_i16().map_err(DynamicError::deserialization)?,
        )),
        TypeDef::Primitive(PrimitiveTypeDef::U16) => Ok(DynamicValue::Uint16(
            reader.read_u16().map_err(DynamicError::deserialization)?,
        )),
        TypeDef::Primitive(PrimitiveTypeDef::I32) => Ok(DynamicValue::Int32(
            reader.read_i32().map_err(DynamicError::deserialization)?,
        )),
        TypeDef::Primitive(PrimitiveTypeDef::U32) => Ok(DynamicValue::Uint32(
            reader.read_u32().map_err(DynamicError::deserialization)?,
        )),
        TypeDef::Primitive(PrimitiveTypeDef::I64) => Ok(DynamicValue::Int64(
            reader.read_i64().map_err(DynamicError::deserialization)?,
        )),
        TypeDef::Primitive(PrimitiveTypeDef::U64) => Ok(DynamicValue::Uint64(
            reader.read_u64().map_err(DynamicError::deserialization)?,
        )),
        TypeDef::Primitive(PrimitiveTypeDef::F32) => Ok(DynamicValue::Float32(
            reader.read_f32().map_err(DynamicError::deserialization)?,
        )),
        TypeDef::Primitive(PrimitiveTypeDef::F64) => Ok(DynamicValue::Float64(
            reader.read_f64().map_err(DynamicError::deserialization)?,
        )),
        TypeDef::String => Ok(DynamicValue::String(
            reader
                .read_string()
                .map_err(DynamicError::deserialization)?,
        )),
        TypeDef::Optional(element) => {
            match reader.read_u32().map_err(DynamicError::deserialization)? {
                0 => Ok(DynamicValue::Optional(None)),
                1 => Ok(DynamicValue::Optional(Some(Box::new(
                    deserialize_shape_value(element, schema, reader)?,
                )))),
                other => Err(DynamicError::deserialization(CdrError::InvalidOptionTag(
                    other,
                ))),
            }
        }
        TypeDef::Sequence { element, length } => match length {
            SequenceLengthDef::Dynamic
                if matches!(element.as_ref(), TypeDef::Primitive(PrimitiveTypeDef::U8)) =>
            {
                Ok(DynamicValue::Bytes(
                    reader
                        .read_byte_sequence()
                        .map_err(DynamicError::deserialization)?
                        .to_vec(),
                ))
            }
            SequenceLengthDef::Dynamic => {
                let len = reader
                    .read_sequence_length()
                    .map_err(DynamicError::deserialization)?;
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
                let variant_index = reader.read_u32().map_err(DynamicError::deserialization)?;
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
            let len = reader
                .read_sequence_length()
                .map_err(DynamicError::deserialization)?;
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

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, sync::Arc};

    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::{
        Message,
        message::{SerdeCdrCodec, WireEncoder},
        schema::SchemaBuilder,
    };

    #[test]
    fn unit_root_roundtrips_without_payload() {
        let schema = Arc::new(<()>::schema());
        let bytes = <() as Message>::Codec::serialize(&()).unwrap();
        assert_eq!(bytes.as_slice(), CDR_HEADER_LE);

        let value = deserialize_cdr_value(&bytes, &schema).unwrap();
        assert_eq!(value, DynamicValue::Unit);
        assert_eq!(serialize_cdr_value(&schema, &value).unwrap(), bytes);
        assert!(value.validate_against(&Arc::new(bool::schema())).is_err());
    }

    #[test]
    fn unit_fields_preserve_cdr_alignment() {
        #[derive(Serialize)]
        struct UnitPacket {
            leading: u8,
            signal: (),
            value: u8,
            aligned: u64,
        }

        let packet = UnitPacket {
            leading: 7,
            signal: (),
            value: 42,
            aligned: 0x1234_5678_1234_5678,
        };
        let mut builder = SchemaBuilder::new();
        let root = builder
            .define_struct(TypeName::new("test::UnitPacket").unwrap(), |fields| {
                fields.field::<u8>("leading")?;
                fields.field::<()>("signal")?;
                fields.field::<u8>("value")?;
                fields.field::<u64>("aligned")?;
                Ok(())
            })
            .unwrap();
        let schema = Arc::new(builder.finish(root).unwrap());
        let bytes = SerdeCdrCodec::<UnitPacket>::serialize(&packet).unwrap();

        let value = deserialize_cdr_value(&bytes, &schema).unwrap();
        assert_eq!(serialize_cdr_value(&schema, &value).unwrap(), bytes);

        let path: ValuePath = ".signal".parse().unwrap();
        path.select_cdr_as::<()>(&bytes, &schema).unwrap();
        let path: ValuePath = ".value".parse().unwrap();
        assert_eq!(
            path.select_cdr_as::<u8>(&bytes, &schema).unwrap(),
            packet.value
        );
        let path: ValuePath = ".aligned".parse().unwrap();
        assert_eq!(
            path.select_cdr_as::<u64>(&bytes, &schema).unwrap(),
            packet.aligned
        );
    }

    #[derive(Serialize, Deserialize, Message)]
    enum Skipped {
        Empty,
        Bytes(Vec<u8>),
        Tuple(i16, String),
        Named { data: Vec<f32> },
    }

    #[derive(Serialize, Deserialize, Message)]
    struct Prefix {
        words: Vec<i32>,
        flag: bool,
        signed: i8,
        shorts: [u16; 2],
        floats: [f64; 2],
        values: Vec<Option<Skipped>>,
        map: BTreeMap<String, Option<[i64; 2]>>,
    }

    #[derive(Serialize, Deserialize, Message)]
    struct Packet {
        leading: u8,
        prefix: Prefix,
        value: u64,
        trailing: Vec<u8>,
    }

    #[test]
    fn skipping_compound_prefixes_preserves_decoder_alignment() {
        let schema = Arc::new(Packet::schema());
        let path: ValuePath = ".value".parse().unwrap();
        for words in [vec![], vec![1, 2, 3]] {
            let packet = Packet {
                leading: 7,
                prefix: Prefix {
                    words,
                    flag: true,
                    signed: -1,
                    shorts: [3, 4],
                    floats: [1.0, f64::NAN],
                    values: vec![
                        None,
                        Some(Skipped::Empty),
                        Some(Skipped::Bytes(vec![0; 65536])),
                        Some(Skipped::Tuple(-1, "prefix".into())),
                        Some(Skipped::Named {
                            data: vec![1.0, 2.0],
                        }),
                    ],
                    map: BTreeMap::from([("a".into(), None), ("b".into(), Some([-1, 2]))]),
                },
                value: 0x1234_5678_1234_5678,
                trailing: vec![],
            };
            let bytes = <Packet as Message>::Codec::serialize(&packet).unwrap();
            assert_eq!(
                path.select_cdr_as::<u64>(&bytes, &schema).unwrap(),
                packet.value
            );
            let mut skipped = CdrReader::<LittleEndian>::new(&bytes[4..]);
            skip_value(&schema.root, &schema, &mut skipped).unwrap();
            let mut decoded = CdrReader::<LittleEndian>::new(&bytes[4..]);
            deserialize_root_value(&schema, &mut decoded).unwrap();
            assert_eq!(skipped.position(), decoded.position());
            assert_eq!(skipped.remaining(), 0);
        }
    }

    #[test]
    fn skipping_keeps_wire_validation_and_typed_errors() {
        let schema = Arc::new(bool::schema());
        let mut reader = CdrReader::<LittleEndian>::new(&[2]);
        assert!(matches!(
            skip_value(&schema.root, &schema, &mut reader),
            Err(CdrSelectionError::Cdr(CdrError::InvalidBool(2)))
        ));
        let schema = Arc::new(Option::<u8>::schema());
        let mut reader = CdrReader::<LittleEndian>::new(&[2, 0, 0, 0]);
        assert!(matches!(
            skip_value(&schema.root, &schema, &mut reader),
            Err(CdrSelectionError::Cdr(CdrError::InvalidOptionTag(2)))
        ));
        let schema = Arc::new(String::schema());
        let mut reader = CdrReader::<LittleEndian>::new(&[2, 0, 0, 0, 255, 0]);
        assert!(matches!(
            skip_value(&schema.root, &schema, &mut reader),
            Err(CdrSelectionError::Cdr(CdrError::Utf8(_)))
        ));
        let mut reader = CdrReader::<LittleEndian>::new(&[2, 0, 0, 0, b'a', 1]);
        assert!(matches!(
            skip_value(&schema.root, &schema, &mut reader),
            Err(CdrSelectionError::Cdr(CdrError::InvalidStringTerminator))
        ));
    }
}
