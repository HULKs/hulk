use std::collections::BTreeMap;
use std::sync::Arc;

use ros_z_schema::{
    EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, LiteralValue, NamedTypeDef,
    PrimitiveTypeDef, RootTypeName, SchemaBundle, SchemaDefinitions, SequenceLengthDef, StructDef,
    TypeDef, TypeName,
};

use super::DynamicError;
use super::schema::{
    FieldSchema, PrimitiveType, RuntimeDynamicEnumPayload, RuntimeDynamicEnumVariant, Schema,
    SequenceLength, TypeShape,
};
use crate::dynamic::DynamicValue;

pub fn schema_to_bundle(root_name: &str, schema: &Schema) -> Result<SchemaBundle, DynamicError> {
    let mut definitions = BTreeMap::new();
    let mut path = Vec::new();
    let root = schema_to_type_def(schema, &mut definitions, &mut path)?;
    let bundle = SchemaBundle {
        root_name: RootTypeName::new(root_name)
            .map_err(|error| DynamicError::InvalidTypeName(error.to_string()))?,
        root,
        definitions: definitions.into(),
    };

    bundle
        .validate()
        .map_err(|error| DynamicError::SerializationError(error.to_string()))?;
    Ok(bundle)
}

pub fn schema_hash_with_root_name(
    root_name: &str,
    schema: &Schema,
) -> Result<crate::entity::SchemaHash, DynamicError> {
    Ok(ros_z_schema::compute_hash(&schema_to_bundle(
        root_name, schema,
    )?))
}

pub fn bundle_to_schema(bundle: &SchemaBundle) -> Result<Schema, DynamicError> {
    bundle
        .validate()
        .map_err(|error| DynamicError::SerializationError(error.to_string()))?;
    type_def_to_schema(&bundle.root, &bundle.definitions)
}

fn schema_to_type_def(
    schema: &Schema,
    definitions: &mut BTreeMap<TypeName, NamedTypeDef>,
    path: &mut Vec<TypeName>,
) -> Result<TypeDef, DynamicError> {
    match schema.as_ref() {
        TypeShape::Struct { name, fields } => {
            collect_struct_definition(name, fields, definitions, path)?;
            Ok(TypeDef::StructRef(name.clone()))
        }
        TypeShape::Enum { name, variants } => {
            collect_enum_definition(name, variants, definitions, path)?;
            Ok(TypeDef::EnumRef(name.clone()))
        }
        TypeShape::Primitive(primitive) => Ok(TypeDef::Primitive(primitive_to_def(*primitive))),
        TypeShape::String => Ok(TypeDef::String),
        TypeShape::Optional(element) => Ok(TypeDef::Optional(Box::new(schema_to_type_def(
            element,
            definitions,
            path,
        )?))),
        TypeShape::Sequence { element, length } => Ok(TypeDef::Sequence {
            element: Box::new(schema_to_type_def(element, definitions, path)?),
            length: match length {
                SequenceLength::Dynamic => SequenceLengthDef::Dynamic,
                SequenceLength::Fixed(length) => SequenceLengthDef::Fixed(*length),
            },
        }),
        TypeShape::Map { key, value } => Ok(TypeDef::Map {
            key: Box::new(schema_to_type_def(key, definitions, path)?),
            value: Box::new(schema_to_type_def(value, definitions, path)?),
        }),
    }
}

fn collect_struct_definition(
    name: &TypeName,
    fields: &[FieldSchema],
    definitions: &mut BTreeMap<TypeName, NamedTypeDef>,
    path: &mut Vec<TypeName>,
) -> Result<(), DynamicError> {
    if path.contains(name) {
        return Err(DynamicError::SerializationError(format!(
            "recursive schema `{name}` is not supported"
        )));
    }

    path.push(name.clone());
    let fields = fields
        .iter()
        .map(|field| runtime_field_to_def(field, definitions, path))
        .collect::<Result<Vec<_>, _>>()?;
    path.pop();

    insert_definition(
        definitions,
        name.clone(),
        NamedTypeDef::Struct(StructDef { fields }),
    )
}

fn collect_enum_definition(
    name: &TypeName,
    variants: &[RuntimeDynamicEnumVariant],
    definitions: &mut BTreeMap<TypeName, NamedTypeDef>,
    path: &mut Vec<TypeName>,
) -> Result<(), DynamicError> {
    if path.contains(name) {
        return Err(DynamicError::SerializationError(format!(
            "recursive schema `{name}` is not supported"
        )));
    }

    path.push(name.clone());
    let variants = variants
        .iter()
        .map(|variant| runtime_enum_variant_to_def(variant, definitions, path))
        .collect::<Result<Vec<_>, _>>()?;
    path.pop();

    insert_definition(
        definitions,
        name.clone(),
        NamedTypeDef::Enum(EnumDef { variants }),
    )
}

fn insert_definition(
    definitions: &mut BTreeMap<TypeName, NamedTypeDef>,
    name: TypeName,
    definition: NamedTypeDef,
) -> Result<(), DynamicError> {
    if let Some(existing) = definitions.get(&name) {
        if existing == &definition {
            return Ok(());
        }
        return Err(DynamicError::SerializationError(format!(
            "conflicting schema definitions for `{name}`"
        )));
    }

    definitions.insert(name, definition);
    Ok(())
}

fn runtime_field_to_def(
    field: &FieldSchema,
    definitions: &mut BTreeMap<TypeName, NamedTypeDef>,
    path: &mut Vec<TypeName>,
) -> Result<FieldDef, DynamicError> {
    let shape = schema_to_type_def(&field.schema, definitions, path)?;
    let default = field
        .default_value
        .as_ref()
        .map(|default| dynamic_value_to_literal_for_shape(default, &shape))
        .transpose()?;
    let mut field_def = FieldDef::new(field.name.clone(), shape);
    if let Some(default) = default {
        field_def = field_def.with_default(default);
    }
    Ok(field_def)
}

fn runtime_enum_variant_to_def(
    variant: &RuntimeDynamicEnumVariant,
    definitions: &mut BTreeMap<TypeName, NamedTypeDef>,
    path: &mut Vec<TypeName>,
) -> Result<EnumVariantDef, DynamicError> {
    Ok(EnumVariantDef::new(
        variant.name.clone(),
        runtime_enum_payload_to_def(&variant.payload, definitions, path)?,
    ))
}

fn runtime_enum_payload_to_def(
    payload: &RuntimeDynamicEnumPayload,
    definitions: &mut BTreeMap<TypeName, NamedTypeDef>,
    path: &mut Vec<TypeName>,
) -> Result<EnumPayloadDef, DynamicError> {
    match payload {
        RuntimeDynamicEnumPayload::Unit => Ok(EnumPayloadDef::Unit),
        RuntimeDynamicEnumPayload::Newtype(schema) => Ok(EnumPayloadDef::Newtype(
            schema_to_type_def(schema, definitions, path)?,
        )),
        RuntimeDynamicEnumPayload::Tuple(schemas) => Ok(EnumPayloadDef::Tuple(
            schemas
                .iter()
                .map(|schema| schema_to_type_def(schema, definitions, path))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        RuntimeDynamicEnumPayload::Struct(fields) => Ok(EnumPayloadDef::Struct(
            fields
                .iter()
                .map(|field| runtime_field_to_def(field, definitions, path))
                .collect::<Result<Vec<_>, _>>()?,
        )),
    }
}

fn primitive_to_def(primitive: PrimitiveType) -> PrimitiveTypeDef {
    match primitive {
        PrimitiveType::Bool => PrimitiveTypeDef::Bool,
        PrimitiveType::I8 => PrimitiveTypeDef::I8,
        PrimitiveType::U8 => PrimitiveTypeDef::U8,
        PrimitiveType::I16 => PrimitiveTypeDef::I16,
        PrimitiveType::U16 => PrimitiveTypeDef::U16,
        PrimitiveType::I32 => PrimitiveTypeDef::I32,
        PrimitiveType::U32 => PrimitiveTypeDef::U32,
        PrimitiveType::I64 => PrimitiveTypeDef::I64,
        PrimitiveType::U64 => PrimitiveTypeDef::U64,
        PrimitiveType::F32 => PrimitiveTypeDef::F32,
        PrimitiveType::F64 => PrimitiveTypeDef::F64,
    }
}

fn primitive_from_def(primitive: PrimitiveTypeDef) -> PrimitiveType {
    match primitive {
        PrimitiveTypeDef::Bool => PrimitiveType::Bool,
        PrimitiveTypeDef::I8 => PrimitiveType::I8,
        PrimitiveTypeDef::U8 => PrimitiveType::U8,
        PrimitiveTypeDef::I16 => PrimitiveType::I16,
        PrimitiveTypeDef::U16 => PrimitiveType::U16,
        PrimitiveTypeDef::I32 => PrimitiveType::I32,
        PrimitiveTypeDef::U32 => PrimitiveType::U32,
        PrimitiveTypeDef::I64 => PrimitiveType::I64,
        PrimitiveTypeDef::U64 => PrimitiveType::U64,
        PrimitiveTypeDef::F32 => PrimitiveType::F32,
        PrimitiveTypeDef::F64 => PrimitiveType::F64,
    }
}

fn type_def_to_schema(
    shape: &TypeDef,
    definitions: &SchemaDefinitions,
) -> Result<Schema, DynamicError> {
    type_def_to_schema_with_path(shape, definitions, &mut Vec::new())
}

fn type_def_to_schema_with_path(
    shape: &TypeDef,
    definitions: &SchemaDefinitions,
    path: &mut Vec<TypeName>,
) -> Result<Schema, DynamicError> {
    Ok(Arc::new(match shape {
        TypeDef::Primitive(primitive) => TypeShape::Primitive(primitive_from_def(*primitive)),
        TypeDef::String => TypeShape::String,
        TypeDef::StructRef(type_name) => {
            named_definition_to_schema(type_name, definitions, true, path)?
        }
        TypeDef::EnumRef(type_name) => {
            named_definition_to_schema(type_name, definitions, false, path)?
        }
        TypeDef::Optional(element) => {
            TypeShape::Optional(type_def_to_schema_with_path(element, definitions, path)?)
        }
        TypeDef::Sequence { element, length } => TypeShape::Sequence {
            element: type_def_to_schema_with_path(element, definitions, path)?,
            length: match length {
                SequenceLengthDef::Dynamic => SequenceLength::Dynamic,
                SequenceLengthDef::Fixed(length) => SequenceLength::Fixed(*length),
            },
        },
        TypeDef::Map { key, value } => TypeShape::Map {
            key: type_def_to_schema_with_path(key, definitions, path)?,
            value: type_def_to_schema_with_path(value, definitions, path)?,
        },
    }))
}

fn named_definition_to_schema(
    type_name: &TypeName,
    definitions: &SchemaDefinitions,
    expect_struct: bool,
    path: &mut Vec<TypeName>,
) -> Result<TypeShape, DynamicError> {
    if path.contains(type_name) {
        return Err(DynamicError::SerializationError(format!(
            "recursive schema `{type_name}` is not supported"
        )));
    }

    let definition = definitions.get(type_name).ok_or_else(|| {
        DynamicError::SerializationError(format!("missing schema definition for {type_name}"))
    })?;

    path.push(type_name.clone());
    let shape = match (definition, expect_struct) {
        (NamedTypeDef::Struct(definition), true) => Ok(TypeShape::Struct {
            name: type_name.clone(),
            fields: struct_def_to_runtime_fields(definition, definitions, path)?,
        }),
        (NamedTypeDef::Enum(definition), false) => Ok(TypeShape::Enum {
            name: type_name.clone(),
            variants: enum_def_to_runtime_variants(definition, definitions, path)?,
        }),
        (NamedTypeDef::Struct(_), false) => Err(DynamicError::SerializationError(format!(
            "schema definition `{type_name}` is not an enum"
        ))),
        (NamedTypeDef::Enum(_), true) => Err(DynamicError::SerializationError(format!(
            "schema definition `{type_name}` is not a struct"
        ))),
    };
    path.pop();
    shape
}

fn struct_def_to_runtime_fields(
    definition: &StructDef,
    definitions: &SchemaDefinitions,
    path: &mut Vec<TypeName>,
) -> Result<Vec<FieldSchema>, DynamicError> {
    definition
        .fields
        .iter()
        .map(|field| {
            Ok(FieldSchema {
                name: field.name.clone(),
                schema: type_def_to_schema_with_path(&field.shape, definitions, path)?,
                default_value: field
                    .default
                    .as_ref()
                    .map(|default| literal_to_dynamic_value(default, &field.shape))
                    .transpose()?,
            })
        })
        .collect()
}

fn enum_def_to_runtime_variants(
    definition: &EnumDef,
    definitions: &SchemaDefinitions,
    path: &mut Vec<TypeName>,
) -> Result<Vec<RuntimeDynamicEnumVariant>, DynamicError> {
    definition
        .variants
        .iter()
        .map(|variant| {
            Ok(RuntimeDynamicEnumVariant::new(
                variant.name.clone(),
                enum_payload_def_to_runtime(&variant.payload, definitions, path)?,
            ))
        })
        .collect()
}

fn enum_payload_def_to_runtime(
    payload: &EnumPayloadDef,
    definitions: &SchemaDefinitions,
    path: &mut Vec<TypeName>,
) -> Result<RuntimeDynamicEnumPayload, DynamicError> {
    match payload {
        EnumPayloadDef::Unit => Ok(RuntimeDynamicEnumPayload::Unit),
        EnumPayloadDef::Newtype(shape) => Ok(RuntimeDynamicEnumPayload::Newtype(
            type_def_to_schema_with_path(shape, definitions, path)?,
        )),
        EnumPayloadDef::Tuple(shapes) => Ok(RuntimeDynamicEnumPayload::Tuple(
            shapes
                .iter()
                .map(|shape| type_def_to_schema_with_path(shape, definitions, path))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        EnumPayloadDef::Struct(fields) => Ok(RuntimeDynamicEnumPayload::Struct(
            fields
                .iter()
                .map(|field| {
                    Ok(FieldSchema {
                        name: field.name.clone(),
                        schema: type_def_to_schema_with_path(&field.shape, definitions, path)?,
                        default_value: field
                            .default
                            .as_ref()
                            .map(|default| literal_to_dynamic_value(default, &field.shape))
                            .transpose()?,
                    })
                })
                .collect::<Result<Vec<_>, DynamicError>>()?,
        )),
    }
}

fn dynamic_value_to_literal(value: &DynamicValue) -> Result<LiteralValue, DynamicError> {
    match value {
        DynamicValue::Bool(value) => Ok(LiteralValue::Bool(*value)),
        DynamicValue::Int8(value) => Ok(LiteralValue::Int(i64::from(*value))),
        DynamicValue::Int16(value) => Ok(LiteralValue::Int(i64::from(*value))),
        DynamicValue::Int32(value) => Ok(LiteralValue::Int(i64::from(*value))),
        DynamicValue::Int64(value) => Ok(LiteralValue::Int(*value)),
        DynamicValue::Uint8(value) => Ok(LiteralValue::UInt(u64::from(*value))),
        DynamicValue::Uint16(value) => Ok(LiteralValue::UInt(u64::from(*value))),
        DynamicValue::Uint32(value) => Ok(LiteralValue::UInt(u64::from(*value))),
        DynamicValue::Uint64(value) => Ok(LiteralValue::UInt(*value)),
        DynamicValue::Float32(value) => Ok(LiteralValue::Float32(*value)),
        DynamicValue::Float64(value) => Ok(LiteralValue::Float64(*value)),
        DynamicValue::String(value) => Ok(LiteralValue::String(value.clone())),
        _ => Err(DynamicError::SerializationError(
            "runtime defaults must be bool/int/uint/float/string literals".to_string(),
        )),
    }
}

fn dynamic_value_to_literal_for_shape(
    value: &DynamicValue,
    shape: &TypeDef,
) -> Result<LiteralValue, DynamicError> {
    match (value, shape) {
        (DynamicValue::Bytes(values), TypeDef::Sequence { element, length: _ })
            if matches!(element.as_ref(), TypeDef::Primitive(PrimitiveTypeDef::U8)) =>
        {
            Ok(LiteralValue::UIntArray(
                values.iter().map(|value| u64::from(*value)).collect(),
            ))
        }
        (DynamicValue::Sequence(values), TypeDef::Sequence { element, length: _ }) => {
            dynamic_array_to_literal(values, element)
        }
        _ => dynamic_value_to_literal(value),
    }
}

fn dynamic_array_to_literal(
    values: &[DynamicValue],
    element: &TypeDef,
) -> Result<LiteralValue, DynamicError> {
    match element {
        TypeDef::Primitive(PrimitiveTypeDef::Bool) => values
            .iter()
            .map(|value| match value {
                DynamicValue::Bool(value) => Ok(*value),
                _ => Err(array_default_error()),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(LiteralValue::BoolArray),
        TypeDef::Primitive(
            PrimitiveTypeDef::I8
            | PrimitiveTypeDef::I16
            | PrimitiveTypeDef::I32
            | PrimitiveTypeDef::I64,
        ) => values
            .iter()
            .map(|value| match value {
                DynamicValue::Int8(value) => Ok(i64::from(*value)),
                DynamicValue::Int16(value) => Ok(i64::from(*value)),
                DynamicValue::Int32(value) => Ok(i64::from(*value)),
                DynamicValue::Int64(value) => Ok(*value),
                _ => Err(array_default_error()),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(LiteralValue::IntArray),
        TypeDef::Primitive(
            PrimitiveTypeDef::U8
            | PrimitiveTypeDef::U16
            | PrimitiveTypeDef::U32
            | PrimitiveTypeDef::U64,
        ) => values
            .iter()
            .map(|value| match value {
                DynamicValue::Uint8(value) => Ok(u64::from(*value)),
                DynamicValue::Uint16(value) => Ok(u64::from(*value)),
                DynamicValue::Uint32(value) => Ok(u64::from(*value)),
                DynamicValue::Uint64(value) => Ok(*value),
                _ => Err(array_default_error()),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(LiteralValue::UIntArray),
        TypeDef::Primitive(PrimitiveTypeDef::F32) => values
            .iter()
            .map(|value| match value {
                DynamicValue::Float32(value) => Ok(*value),
                _ => Err(array_default_error()),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(LiteralValue::Float32Array),
        TypeDef::Primitive(PrimitiveTypeDef::F64) => values
            .iter()
            .map(|value| match value {
                DynamicValue::Float64(value) => Ok(*value),
                _ => Err(array_default_error()),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(LiteralValue::Float64Array),
        TypeDef::String => values
            .iter()
            .map(|value| match value {
                DynamicValue::String(value) => Ok(value.clone()),
                _ => Err(array_default_error()),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(LiteralValue::StringArray),
        _ => Err(array_default_error()),
    }
}

fn array_default_error() -> DynamicError {
    DynamicError::SerializationError(
        "runtime array defaults must contain only primitive/string literal values matching the field shape"
            .to_string(),
    )
}

fn literal_to_dynamic_value(
    value: &LiteralValue,
    shape: &TypeDef,
) -> Result<DynamicValue, DynamicError> {
    match (value, shape) {
        (LiteralValue::Bool(value), TypeDef::Primitive(PrimitiveTypeDef::Bool)) => {
            Ok(DynamicValue::Bool(*value))
        }
        (LiteralValue::Int(value), TypeDef::Primitive(primitive)) => match primitive {
            PrimitiveTypeDef::I8 => i8::try_from(*value)
                .map(DynamicValue::Int8)
                .map_err(|error| DynamicError::SerializationError(error.to_string())),
            PrimitiveTypeDef::I16 => i16::try_from(*value)
                .map(DynamicValue::Int16)
                .map_err(|error| DynamicError::SerializationError(error.to_string())),
            PrimitiveTypeDef::I32 => i32::try_from(*value)
                .map(DynamicValue::Int32)
                .map_err(|error| DynamicError::SerializationError(error.to_string())),
            PrimitiveTypeDef::I64 => Ok(DynamicValue::Int64(*value)),
            _ => Err(DynamicError::SerializationError(format!(
                "invalid signed default for primitive shape `{}`",
                primitive.as_str()
            ))),
        },
        (LiteralValue::UInt(value), TypeDef::Primitive(primitive)) => match primitive {
            PrimitiveTypeDef::U8 => u8::try_from(*value)
                .map(DynamicValue::Uint8)
                .map_err(|error| DynamicError::SerializationError(error.to_string())),
            PrimitiveTypeDef::U16 => u16::try_from(*value)
                .map(DynamicValue::Uint16)
                .map_err(|error| DynamicError::SerializationError(error.to_string())),
            PrimitiveTypeDef::U32 => u32::try_from(*value)
                .map(DynamicValue::Uint32)
                .map_err(|error| DynamicError::SerializationError(error.to_string())),
            PrimitiveTypeDef::U64 => Ok(DynamicValue::Uint64(*value)),
            _ => Err(DynamicError::SerializationError(format!(
                "invalid unsigned default for primitive shape `{}`",
                primitive.as_str()
            ))),
        },
        (LiteralValue::Float32(value), TypeDef::Primitive(PrimitiveTypeDef::F32)) => {
            Ok(DynamicValue::Float32(*value))
        }
        (LiteralValue::Float64(value), TypeDef::Primitive(PrimitiveTypeDef::F64)) => {
            Ok(DynamicValue::Float64(*value))
        }
        (LiteralValue::String(value), TypeDef::String) => Ok(DynamicValue::String(value.clone())),
        (LiteralValue::BoolArray(values), TypeDef::Sequence { element, length: _ })
            if matches!(element.as_ref(), TypeDef::Primitive(PrimitiveTypeDef::Bool)) =>
        {
            Ok(DynamicValue::Sequence(
                values.iter().copied().map(DynamicValue::Bool).collect(),
            ))
        }
        (LiteralValue::IntArray(values), TypeDef::Sequence { element, length: _ }) => {
            signed_array_literal_to_dynamic(values, element)
        }
        (LiteralValue::UIntArray(values), TypeDef::Sequence { element, length }) => {
            unsigned_array_literal_to_dynamic(values, element, *length)
        }
        (LiteralValue::Float32Array(values), TypeDef::Sequence { element, length: _ })
            if matches!(element.as_ref(), TypeDef::Primitive(PrimitiveTypeDef::F32)) =>
        {
            Ok(DynamicValue::Sequence(
                values.iter().copied().map(DynamicValue::Float32).collect(),
            ))
        }
        (LiteralValue::Float64Array(values), TypeDef::Sequence { element, length: _ })
            if matches!(element.as_ref(), TypeDef::Primitive(PrimitiveTypeDef::F64)) =>
        {
            Ok(DynamicValue::Sequence(
                values.iter().copied().map(DynamicValue::Float64).collect(),
            ))
        }
        (LiteralValue::StringArray(values), TypeDef::Sequence { element, length: _ })
            if matches!(element.as_ref(), TypeDef::String) =>
        {
            Ok(DynamicValue::Sequence(
                values.iter().cloned().map(DynamicValue::String).collect(),
            ))
        }
        _ => Err(DynamicError::SerializationError(
            "default does not match runtime field shape".to_string(),
        )),
    }
}

fn signed_array_literal_to_dynamic(
    values: &[i64],
    element: &TypeDef,
) -> Result<DynamicValue, DynamicError> {
    let values = match element {
        TypeDef::Primitive(PrimitiveTypeDef::I8) => values
            .iter()
            .map(|value| i8::try_from(*value).map(DynamicValue::Int8))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| DynamicError::SerializationError(error.to_string()))?,
        TypeDef::Primitive(PrimitiveTypeDef::I16) => values
            .iter()
            .map(|value| i16::try_from(*value).map(DynamicValue::Int16))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| DynamicError::SerializationError(error.to_string()))?,
        TypeDef::Primitive(PrimitiveTypeDef::I32) => values
            .iter()
            .map(|value| i32::try_from(*value).map(DynamicValue::Int32))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| DynamicError::SerializationError(error.to_string()))?,
        TypeDef::Primitive(PrimitiveTypeDef::I64) => {
            values.iter().copied().map(DynamicValue::Int64).collect()
        }
        _ => {
            return Err(DynamicError::SerializationError(
                "default does not match runtime field shape".to_string(),
            ));
        }
    };
    Ok(DynamicValue::Sequence(values))
}

fn unsigned_array_literal_to_dynamic(
    values: &[u64],
    element: &TypeDef,
    length: SequenceLengthDef,
) -> Result<DynamicValue, DynamicError> {
    match element {
        TypeDef::Primitive(PrimitiveTypeDef::U8) => {
            let values = values
                .iter()
                .map(|value| u8::try_from(*value))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| DynamicError::SerializationError(error.to_string()))?;
            match length {
                SequenceLengthDef::Dynamic => Ok(DynamicValue::Bytes(values)),
                SequenceLengthDef::Fixed(_) => Ok(DynamicValue::Sequence(
                    values.into_iter().map(DynamicValue::Uint8).collect(),
                )),
            }
        }
        TypeDef::Primitive(PrimitiveTypeDef::U16) => values
            .iter()
            .map(|value| u16::try_from(*value).map(DynamicValue::Uint16))
            .collect::<Result<Vec<_>, _>>()
            .map(DynamicValue::Sequence)
            .map_err(|error| DynamicError::SerializationError(error.to_string())),
        TypeDef::Primitive(PrimitiveTypeDef::U32) => values
            .iter()
            .map(|value| u32::try_from(*value).map(DynamicValue::Uint32))
            .collect::<Result<Vec<_>, _>>()
            .map(DynamicValue::Sequence)
            .map_err(|error| DynamicError::SerializationError(error.to_string())),
        TypeDef::Primitive(PrimitiveTypeDef::U64) => Ok(DynamicValue::Sequence(
            values.iter().copied().map(DynamicValue::Uint64).collect(),
        )),
        _ => Err(DynamicError::SerializationError(
            "default does not match runtime field shape".to_string(),
        )),
    }
}
