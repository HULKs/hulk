//! CDR serialization for dynamic messages.
//!
//! This module uses the low-level primitives from `ros-z-cdr` for CDR
//! serialization and deserialization of dynamic messages.

use ros_z_cdr::{
    CdrBuffer, CdrDeserializer, CdrReader, CdrWriter, Error as CdrError, LittleEndian,
};
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
    let mut buffer = Vec::with_capacity(256);
    serialize_cdr_into(message, &mut buffer)?;
    Ok(buffer)
}

pub(super) fn serialize_cdr_into<B: CdrBuffer>(
    message: &DynamicStruct,
    buffer: &mut B,
) -> Result<(), DynamicError> {
    let schema = message.schema_ref();
    let definition = crate::dynamic::value::validate_struct(message, &schema.root, schema)?;
    buffer.extend_from_slice(&CDR_HEADER_LE);
    serialize_struct_fields(
        message,
        &definition.fields,
        schema,
        &mut CdrWriter::new(buffer),
    )
}

/// Serialize a dynamic root value to CDR bytes.
pub fn serialize_cdr_value(schema: &Schema, value: &DynamicValue) -> Result<Vec<u8>, DynamicError> {
    let mut buffer = Vec::with_capacity(256);
    serialize_cdr_value_into(schema, value, &mut buffer)?;
    Ok(buffer)
}

pub(crate) fn serialize_cdr_value_into<B: CdrBuffer>(
    schema: &Schema,
    value: &DynamicValue,
    buffer: &mut B,
) -> Result<(), DynamicError> {
    value.validate_against(schema)?;
    write_cdr_value(schema, value, buffer)
}

pub(crate) fn write_cdr_value<B: CdrBuffer>(
    schema: &Schema,
    value: &DynamicValue,
    buffer: &mut B,
) -> Result<(), DynamicError> {
    buffer.extend_from_slice(&CDR_HEADER_LE);
    let mut writer = CdrWriter::<LittleEndian, B>::new(buffer);
    serialize_root_value(value, schema, &mut writer)
}

/// Serialize a dynamic message to a ZBuf.
pub fn serialize_cdr_to_zbuf(message: &DynamicStruct) -> Result<ZBuf, DynamicError> {
    let mut buffer = ros_z_cdr::ZBufWriter::with_capacity(256);
    serialize_cdr_into(message, &mut buffer)?;
    Ok(buffer.into_zbuf())
}

/// Deserialize a dynamic message from CDR bytes.
pub fn deserialize_cdr(data: &[u8], schema: &Schema) -> Result<DynamicStruct, DynamicError> {
    match deserialize_cdr_value(data, schema)? {
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
    deserialize_cdr_reader(CdrReader::new(data), schema)
}

pub(crate) fn deserialize_cdr_reader(
    mut reader: CdrReader<LittleEndian>,
    schema: &Schema,
) -> Result<DynamicValue, DynamicError> {
    let mut header = [0; 4];
    reader
        .read_into(&mut header)
        .map_err(DynamicError::deserialization)?;
    let representation_identifier = &header[0..2];
    if representation_identifier != [0x00, 0x01] {
        return Err(DynamicError::DeserializationError(format!(
            "Expected CDR_LE encapsulation ({:?}), found {:?}",
            [0x00, 0x01],
            representation_identifier
        )));
    }

    reader.reset_origin();
    deserialize_root_value(schema, &mut reader)
}

fn serialize_root_value<B: CdrBuffer>(
    value: &DynamicValue,
    schema: &Schema,
    writer: &mut CdrWriter<LittleEndian, B>,
) -> Result<(), DynamicError> {
    serialize_shape_value(value, &schema.root, schema, writer)
}

// Both public serialization entry points validate the full tree before encoding.
fn serialize_struct_fields<B: CdrBuffer>(
    value: &DynamicStruct,
    fields: &[FieldDef],
    schema: &Schema,
    writer: &mut CdrWriter<LittleEndian, B>,
) -> Result<(), DynamicError> {
    for (field, field_value) in fields.iter().zip(value.values().iter()) {
        serialize_shape_value(field_value, &field.shape, schema, writer)?;
    }
    Ok(())
}

fn serialize_shape_value<B: CdrBuffer>(
    value: &DynamicValue,
    shape: &TypeDef,
    schema: &Schema,
    writer: &mut CdrWriter<LittleEndian, B>,
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
            if matches!(length, SequenceLengthDef::Dynamic) {
                writer.write_sequence_length(bytes.len());
            }
            writer.buffer_mut().append_zbuf(&ZBuf::from(bytes.clone()));
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

fn collection_capacity<T>(len: usize) -> usize {
    len.min(1024 * 1024 / std::mem::size_of::<T>().max(1))
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
                Ok(DynamicValue::Bytes({
                    let len = reader.read_u32().map_err(DynamicError::deserialization)? as usize;
                    reader
                        .read_zslice(len)
                        .map_err(DynamicError::deserialization)?
                }))
            }
            SequenceLengthDef::Dynamic => {
                let len = reader
                    .read_sequence_length()
                    .map_err(DynamicError::deserialization)?;
                let mut values = Vec::with_capacity(collection_capacity::<DynamicValue>(len));
                for _ in 0..len {
                    values.push(deserialize_shape_value(element, schema, reader)?);
                }
                Ok(DynamicValue::Sequence(values))
            }
            SequenceLengthDef::Fixed(len) => {
                let mut values = Vec::with_capacity(collection_capacity::<DynamicValue>(*len));
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
            let mut entries =
                Vec::with_capacity(collection_capacity::<(DynamicValue, DynamicValue)>(len));
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

fn serialize_runtime_enum_payload<B: CdrBuffer>(
    payload: &EnumPayloadValue,
    schema: &EnumPayloadDef,
    bundle: &Schema,
    writer: &mut CdrWriter<LittleEndian, B>,
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
    use crate::{Message, message::WireEncoder, schema::SchemaBuilder};

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
        let mut bytes = CDR_HEADER_LE.to_vec();
        bytes.extend(ros_z_cdr::to_vec::<_, LittleEndian>(&packet, 256).unwrap());

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

    #[derive(Serialize, Deserialize, Message)]
    struct SerializationNested {
        fixed: [u16; 2],
    }

    #[derive(Serialize, Deserialize, Message)]
    enum SerializationEnum {
        Tuple(SerializationNested, u8),
        Named { nested: SerializationNested },
    }

    #[derive(Serialize, Deserialize, Message)]
    struct SerializationRoot {
        nested: SerializationNested,
        variant: SerializationEnum,
    }

    fn serialization_fixture(named: bool) -> (DynamicStruct, Vec<u8>) {
        let packet = SerializationRoot {
            nested: SerializationNested { fixed: [1, 2] },
            variant: if named {
                SerializationEnum::Named {
                    nested: SerializationNested { fixed: [3, 4] },
                }
            } else {
                SerializationEnum::Tuple(SerializationNested { fixed: [5, 6] }, 7)
            },
        };
        let schema = Arc::new(SerializationRoot::schema());
        let bytes = <SerializationRoot as Message>::Codec::serialize(&packet).unwrap();
        (deserialize_cdr(&bytes, &schema).unwrap(), bytes.to_vec())
    }

    fn assert_serialization_rejected(message: DynamicStruct, expected: &str) {
        let schema = message.schema_arc();
        let struct_error = serialize_cdr(&message).unwrap_err().to_string();
        let mut buffer = vec![1, 2, 3];
        let buffer_error = message
            .serialize_to_buf(super::super::SerializationFormat::Cdr, &mut buffer)
            .unwrap_err()
            .to_string();
        assert_eq!(struct_error, buffer_error);
        assert!(buffer.is_empty());
        let value_error = serialize_cdr_value(&schema, &DynamicValue::Struct(Box::new(message)))
            .unwrap_err()
            .to_string();
        assert!(struct_error.contains(expected), "{struct_error}");
        assert_eq!(struct_error, value_error);
    }

    #[test]
    fn borrowed_struct_serialization_matches_root_value_and_typed_bytes() {
        for named in [false, true] {
            let (message, expected) = serialization_fixture(named);
            assert_eq!(serialize_cdr(&message).unwrap(), expected);
            let mut buffer = Vec::with_capacity(expected.len() + 128);
            let allocation = buffer.as_ptr();
            for _ in 0..2 {
                message
                    .serialize_to_buf(super::super::SerializationFormat::Cdr, &mut buffer)
                    .unwrap();
                assert_eq!(buffer.as_ptr(), allocation);
                assert_eq!(buffer, expected);
            }
            assert_eq!(
                serialize_cdr_value(
                    &message.schema_arc(),
                    &DynamicValue::Struct(Box::new(message.clone())),
                )
                .unwrap(),
                expected
            );
        }
    }

    #[test]
    fn dynamic_collection_reservations_are_bounded_and_can_grow() {
        const BUDGET: usize = 1024 * 1024;
        assert_eq!(collection_capacity::<DynamicValue>(3), 3);
        assert_eq!(
            collection_capacity::<DynamicValue>(usize::MAX),
            BUDGET / std::mem::size_of::<DynamicValue>()
        );
        assert_eq!(
            collection_capacity::<(DynamicValue, DynamicValue)>(usize::MAX),
            BUDGET / std::mem::size_of::<(DynamicValue, DynamicValue)>()
        );
        let element = TypeDef::Primitive(PrimitiveTypeDef::U32);
        let mut truncated = CDR_HEADER_LE.to_vec();
        truncated.extend_from_slice(&100_000_000u32.to_le_bytes());
        for shape in [
            TypeDef::Sequence {
                element: Box::new(element.clone()),
                length: SequenceLengthDef::Dynamic,
            },
            TypeDef::Sequence {
                element: Box::new(element.clone()),
                length: SequenceLengthDef::Fixed(100_000_000),
            },
            TypeDef::Map {
                key: Box::new(element.clone()),
                value: Box::new(element),
            },
        ] {
            let schema = Arc::new(ros_z_schema::SchemaBundle::new(shape).unwrap());
            assert!(deserialize_cdr_value(&truncated[..4], &schema).is_err());
            assert!(deserialize_cdr_value(&truncated, &schema).is_err());
        }
        let values = vec![42u32; collection_capacity::<DynamicValue>(usize::MAX) + 1];
        let wire = <Vec<u32> as Message>::Codec::serialize(&values).unwrap();
        let schema = Arc::new(Vec::<u32>::schema());
        let decoded = deserialize_cdr_value(&wire, &schema).unwrap();
        let DynamicValue::Sequence(items) = &decoded else {
            panic!("expected sequence")
        };
        assert_eq!(items.len(), values.len());
        assert_eq!(serialize_cdr_value(&schema, &decoded).unwrap(), wire);
    }

    #[test]
    fn serialization_rejects_mutated_nested_structs_and_fixed_lengths() {
        for len in [0, 2] {
            let (mut message, _) = serialization_fixture(false);
            let DynamicValue::Struct(nested) = &mut message.values_mut()[0] else {
                panic!("expected nested struct");
            };
            nested.values_mut().resize(len, DynamicValue::Bool(false));
            assert_serialization_rejected(message, "struct field count mismatch");
        }

        let (mut message, _) = serialization_fixture(false);
        let DynamicValue::Struct(nested) = &mut message.values_mut()[0] else {
            panic!("expected nested struct");
        };
        nested.values_mut()[0] = DynamicValue::Bool(false);
        assert_serialization_rejected(message, "dynamic value does not match schema");

        for len in [1, 3] {
            let (mut message, _) = serialization_fixture(false);
            let DynamicValue::Struct(nested) = &mut message.values_mut()[0] else {
                panic!("expected nested struct");
            };
            nested.values_mut()[0] = DynamicValue::Sequence(vec![DynamicValue::Uint16(1); len]);
            assert_serialization_rejected(message, "fixed sequence expected 2 values");
        }
    }

    #[test]
    fn serialization_rejects_mutated_enum_payload_lengths_and_nested_types() {
        for named in [false, true] {
            for extra in [false, true] {
                let (mut message, _) = serialization_fixture(named);
                let DynamicValue::Enum(value) = &mut message.values_mut()[1] else {
                    panic!("expected enum");
                };
                match &mut value.payload {
                    EnumPayloadValue::Tuple(values) => {
                        if extra {
                            values.push(DynamicValue::Bool(false));
                        } else {
                            values.pop();
                        }
                    }
                    EnumPayloadValue::Struct(values) => {
                        if extra {
                            values.push(values[0].clone());
                        } else {
                            values.pop();
                        }
                    }
                    _ => panic!("expected tuple or struct payload"),
                }
                assert_serialization_rejected(message, "payload length mismatch");
            }

            let (mut message, _) = serialization_fixture(named);
            let DynamicValue::Enum(value) = &mut message.values_mut()[1] else {
                panic!("expected enum");
            };
            let nested = match &mut value.payload {
                EnumPayloadValue::Tuple(values) => &mut values[0],
                EnumPayloadValue::Struct(values) => &mut values[0].value,
                _ => panic!("expected tuple or struct payload"),
            };
            let DynamicValue::Struct(nested) = nested else {
                panic!("expected nested struct");
            };
            nested.values_mut()[0] = DynamicValue::Bool(false);
            assert_serialization_rejected(message, "dynamic value does not match schema");
        }
    }

    #[test]
    fn serialization_rejects_root_name_mismatch_even_with_matching_fields() {
        use ros_z_schema::{SchemaBundle, StructDef};

        let root_name = TypeName::new("test::Root").unwrap();
        let other_name = TypeName::new("test::OtherRoot").unwrap();
        let definition = TypeDefinition::Struct(StructDef {
            fields: vec![FieldDef::new(
                "next",
                TypeDef::Optional(Box::new(TypeDef::Named(other_name.clone()))),
            )],
        });
        let schema = Arc::new(SchemaBundle {
            root: TypeDef::Named(root_name.clone()),
            definitions: [
                (root_name, definition.clone()),
                (other_name.clone(), definition),
            ]
            .into(),
        });
        let other =
            DynamicStruct::new(schema, other_name, vec![DynamicValue::Optional(None)]).unwrap();
        assert_serialization_rejected(other, "struct type mismatch: expected");
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
