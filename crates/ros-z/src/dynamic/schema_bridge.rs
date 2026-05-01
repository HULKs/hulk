use std::collections::BTreeMap;
use std::sync::Arc;

use ros_z_schema::{
    EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, FieldPrimitive, FieldShape, LiteralValue,
    SchemaBundle, StructDef, TypeDef, TypeName,
};

use super::{
    DynamicError, EnumPayloadSchema, EnumSchema, EnumVariantSchema, FieldSchema, FieldType,
    MessageSchema,
};
use crate::dynamic::DynamicValue;

pub fn message_schema_to_bundle(schema: &MessageSchema) -> Result<SchemaBundle, DynamicError> {
    let mut definitions = BTreeMap::new();
    collect_message_schema(schema, &mut definitions)?;

    let mut builder = SchemaBundle::builder(schema.type_name_str());
    for (type_name, definition) in definitions {
        builder = builder.definition(type_name, definition);
    }

    builder
        .build()
        .map_err(|error| DynamicError::SerializationError(error.to_string()))
}

pub fn bundle_to_message_schema(bundle: &SchemaBundle) -> Result<Arc<MessageSchema>, DynamicError> {
    let root = bundle.definitions.get(&bundle.root).ok_or_else(|| {
        DynamicError::SerializationError("schema bundle missing root definition".to_string())
    })?;

    match root {
        TypeDef::Struct(struct_def) => {
            build_runtime_message_schema(&bundle.definitions, &bundle.root, struct_def)
        }
        TypeDef::Enum(_) => Err(DynamicError::SerializationError(
            "schema bundle root must be a message struct definition".to_string(),
        )),
    }
}

fn collect_message_schema(
    schema: &MessageSchema,
    definitions: &mut BTreeMap<String, TypeDef>,
) -> Result<(), DynamicError> {
    if definitions.contains_key(schema.type_name_str()) {
        return Ok(());
    }

    let fields = schema
        .fields()
        .iter()
        .map(|field| field_schema_to_def(field, schema.type_name_str(), definitions))
        .collect::<Result<Vec<_>, _>>()?;

    definitions.insert(
        schema.type_name_str().to_string(),
        TypeDef::Struct(StructDef { fields }),
    );
    Ok(())
}

fn field_schema_to_def(
    field: &FieldSchema,
    parent_type_name: &str,
    definitions: &mut BTreeMap<String, TypeDef>,
) -> Result<FieldDef, DynamicError> {
    let shape = field_type_to_shape(&field.field_type, parent_type_name, definitions)?;
    let mut field_def = FieldDef::new(field.name.clone(), shape);

    if let Some(default) = &field.default_value {
        field_def = field_def.with_default(dynamic_value_to_literal(default)?);
    }

    Ok(field_def)
}

fn enum_variant_to_def(
    variant: &EnumVariantSchema,
    definitions: &mut BTreeMap<String, TypeDef>,
) -> Result<EnumVariantDef, DynamicError> {
    Ok(EnumVariantDef::new(
        variant.name.clone(),
        enum_payload_to_def(&variant.payload, definitions)?,
    ))
}

fn enum_payload_to_def(
    payload: &EnumPayloadSchema,
    definitions: &mut BTreeMap<String, TypeDef>,
) -> Result<EnumPayloadDef, DynamicError> {
    match payload {
        EnumPayloadSchema::Unit => Ok(EnumPayloadDef::Unit),
        EnumPayloadSchema::Newtype(field_type) => Ok(EnumPayloadDef::Newtype(field_type_to_shape(
            field_type,
            "",
            definitions,
        )?)),
        EnumPayloadSchema::Tuple(field_types) => Ok(EnumPayloadDef::Tuple(
            field_types
                .iter()
                .map(|field_type| field_type_to_shape(field_type, "", definitions))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        EnumPayloadSchema::Struct(fields) => Ok(EnumPayloadDef::Struct(
            fields
                .iter()
                .map(|field| field_schema_to_def(field, "", definitions))
                .collect::<Result<Vec<_>, _>>()?,
        )),
    }
}

fn field_type_to_shape(
    field_type: &FieldType,
    parent_type_name: &str,
    definitions: &mut BTreeMap<String, TypeDef>,
) -> Result<FieldShape, DynamicError> {
    match field_type {
        FieldType::Bool => Ok(FieldShape::Primitive(FieldPrimitive::Bool)),
        FieldType::Int8 => Ok(FieldShape::Primitive(FieldPrimitive::I8)),
        FieldType::Int16 => Ok(FieldShape::Primitive(FieldPrimitive::I16)),
        FieldType::Int32 => Ok(FieldShape::Primitive(FieldPrimitive::I32)),
        FieldType::Int64 => Ok(FieldShape::Primitive(FieldPrimitive::I64)),
        FieldType::Uint8 => Ok(FieldShape::Primitive(FieldPrimitive::U8)),
        FieldType::Uint16 => Ok(FieldShape::Primitive(FieldPrimitive::U16)),
        FieldType::Uint32 => Ok(FieldShape::Primitive(FieldPrimitive::U32)),
        FieldType::Uint64 => Ok(FieldShape::Primitive(FieldPrimitive::U64)),
        FieldType::Float32 => Ok(FieldShape::Primitive(FieldPrimitive::F32)),
        FieldType::Float64 => Ok(FieldShape::Primitive(FieldPrimitive::F64)),
        FieldType::String => Ok(FieldShape::String),
        FieldType::BoundedString(max) => Ok(FieldShape::BoundedString {
            maximum_length: *max,
        }),
        FieldType::Message(schema) => {
            collect_message_schema(schema, definitions)?;
            Ok(FieldShape::Named(parse_type_name(schema.type_name_str())?))
        }
        FieldType::Optional(inner) => Ok(FieldShape::Optional {
            element: Box::new(field_type_to_shape(inner, parent_type_name, definitions)?),
        }),
        FieldType::Array(inner, len) => Ok(FieldShape::Array {
            element: Box::new(field_type_to_shape(inner, parent_type_name, definitions)?),
            length: *len,
        }),
        FieldType::Sequence(inner) => Ok(FieldShape::Sequence {
            element: Box::new(field_type_to_shape(inner, parent_type_name, definitions)?),
        }),
        FieldType::BoundedSequence(inner, max) => Ok(FieldShape::BoundedSequence {
            element: Box::new(field_type_to_shape(inner, parent_type_name, definitions)?),
            maximum_length: *max,
        }),
        FieldType::Map(key, value) => Ok(FieldShape::Map {
            key: Box::new(field_type_to_shape(key, parent_type_name, definitions)?),
            value: Box::new(field_type_to_shape(value, parent_type_name, definitions)?),
        }),
        FieldType::Enum(schema) => {
            let definition_type_name = if schema.type_name == parent_type_name {
                synthetic_enum_type_name(parent_type_name)
            } else {
                schema.type_name.clone()
            };
            collect_enum_schema_as(schema, &definition_type_name, definitions)?;
            Ok(FieldShape::Named(parse_type_name(&definition_type_name)?))
        }
    }
}

fn collect_enum_schema_as(
    schema: &EnumSchema,
    definition_type_name: &str,
    definitions: &mut BTreeMap<String, TypeDef>,
) -> Result<(), DynamicError> {
    if definitions.contains_key(definition_type_name) {
        return Ok(());
    }

    let variants = schema
        .variants
        .iter()
        .map(|variant| enum_variant_to_def(variant, definitions))
        .collect::<Result<Vec<_>, _>>()?;

    definitions.insert(
        definition_type_name.to_string(),
        TypeDef::Enum(EnumDef { variants }),
    );
    Ok(())
}

fn build_runtime_message_schema(
    definitions: &BTreeMap<TypeName, TypeDef>,
    type_name: &TypeName,
    struct_def: &StructDef,
) -> Result<Arc<MessageSchema>, DynamicError> {
    let fields = struct_def
        .fields
        .iter()
        .map(|field| field_def_to_runtime(definitions, field))
        .collect::<Result<Vec<_>, _>>()?;

    let mut builder = MessageSchema::builder(type_name.as_str());
    for field in fields {
        builder = if let Some(default) = field.default_value {
            builder.field_with_default(&field.name, field.field_type, default)
        } else {
            builder.field(&field.name, field.field_type)
        };
    }
    builder.build()
}

fn build_enum_schema(
    definitions: &BTreeMap<TypeName, TypeDef>,
    type_name: &TypeName,
    enum_def: &EnumDef,
) -> Result<EnumSchema, DynamicError> {
    let variants = enum_def
        .variants
        .iter()
        .map(|variant| {
            Ok(EnumVariantSchema::new(
                variant.name.clone(),
                enum_payload_to_runtime(definitions, &variant.payload)?,
            ))
        })
        .collect::<Result<Vec<_>, DynamicError>>()?;

    Ok(EnumSchema::new(logical_enum_type_name(type_name), variants))
}

fn enum_payload_to_runtime(
    definitions: &BTreeMap<TypeName, TypeDef>,
    payload: &EnumPayloadDef,
) -> Result<EnumPayloadSchema, DynamicError> {
    match payload {
        EnumPayloadDef::Unit => Ok(EnumPayloadSchema::Unit),
        EnumPayloadDef::Newtype(shape) => Ok(EnumPayloadSchema::Newtype(Box::new(
            shape_to_field_type(definitions, shape)?,
        ))),
        EnumPayloadDef::Tuple(shapes) => Ok(EnumPayloadSchema::Tuple(
            shapes
                .iter()
                .map(|shape| shape_to_field_type(definitions, shape))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        EnumPayloadDef::Struct(fields) => Ok(EnumPayloadSchema::Struct(
            fields
                .iter()
                .map(|field| field_def_to_runtime(definitions, field))
                .collect::<Result<Vec<_>, _>>()?,
        )),
    }
}

fn field_def_to_runtime(
    definitions: &BTreeMap<TypeName, TypeDef>,
    field: &FieldDef,
) -> Result<FieldSchema, DynamicError> {
    Ok(FieldSchema {
        name: field.name.clone(),
        field_type: shape_to_field_type(definitions, &field.shape)?,
        default_value: field
            .default
            .as_ref()
            .map(|default| literal_to_dynamic_value(default, &field.shape))
            .transpose()?,
    })
}

fn shape_to_field_type(
    definitions: &BTreeMap<TypeName, TypeDef>,
    shape: &FieldShape,
) -> Result<FieldType, DynamicError> {
    match shape {
        FieldShape::Primitive(primitive) => Ok(primitive_field_type(*primitive)),
        FieldShape::String => Ok(FieldType::String),
        FieldShape::BoundedString { maximum_length } => {
            Ok(FieldType::BoundedString(*maximum_length))
        }
        FieldShape::Named(type_name) => {
            let definition = definitions.get(type_name).ok_or_else(|| {
                DynamicError::SerializationError(format!(
                    "missing schema definition for {type_name}"
                ))
            })?;

            match definition {
                TypeDef::Struct(struct_def) => Ok(FieldType::Message(
                    build_runtime_message_schema(definitions, type_name, struct_def)?,
                )),
                TypeDef::Enum(enum_def) => Ok(FieldType::Enum(Arc::new(build_enum_schema(
                    definitions,
                    type_name,
                    enum_def,
                )?))),
            }
        }
        FieldShape::Optional { element } => Ok(FieldType::Optional(Box::new(shape_to_field_type(
            definitions,
            element,
        )?))),
        FieldShape::Array { element, length } => Ok(FieldType::Array(
            Box::new(shape_to_field_type(definitions, element)?),
            *length,
        )),
        FieldShape::Sequence { element } => Ok(FieldType::Sequence(Box::new(shape_to_field_type(
            definitions,
            element,
        )?))),
        FieldShape::BoundedSequence {
            element,
            maximum_length,
        } => Ok(FieldType::BoundedSequence(
            Box::new(shape_to_field_type(definitions, element)?),
            *maximum_length,
        )),
        FieldShape::Map { key, value } => Ok(FieldType::Map(
            Box::new(shape_to_field_type(definitions, key)?),
            Box::new(shape_to_field_type(definitions, value)?),
        )),
    }
}

fn primitive_field_type(primitive: FieldPrimitive) -> FieldType {
    match primitive {
        FieldPrimitive::Bool => FieldType::Bool,
        FieldPrimitive::I8 => FieldType::Int8,
        FieldPrimitive::I16 => FieldType::Int16,
        FieldPrimitive::I32 => FieldType::Int32,
        FieldPrimitive::I64 => FieldType::Int64,
        FieldPrimitive::U8 => FieldType::Uint8,
        FieldPrimitive::U16 => FieldType::Uint16,
        FieldPrimitive::U32 => FieldType::Uint32,
        FieldPrimitive::U64 => FieldType::Uint64,
        FieldPrimitive::F32 => FieldType::Float32,
        FieldPrimitive::F64 => FieldType::Float64,
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

fn literal_to_dynamic_value(
    value: &LiteralValue,
    shape: &FieldShape,
) -> Result<DynamicValue, DynamicError> {
    match (value, shape) {
        (LiteralValue::Bool(value), FieldShape::Primitive(FieldPrimitive::Bool)) => {
            Ok(DynamicValue::Bool(*value))
        }
        (LiteralValue::Int(value), FieldShape::Primitive(primitive)) => match primitive {
            FieldPrimitive::I8 => i8::try_from(*value)
                .map(DynamicValue::Int8)
                .map_err(|error| DynamicError::SerializationError(error.to_string())),
            FieldPrimitive::I16 => i16::try_from(*value)
                .map(DynamicValue::Int16)
                .map_err(|error| DynamicError::SerializationError(error.to_string())),
            FieldPrimitive::I32 => i32::try_from(*value)
                .map(DynamicValue::Int32)
                .map_err(|error| DynamicError::SerializationError(error.to_string())),
            FieldPrimitive::I64 => Ok(DynamicValue::Int64(*value)),
            _ => Err(DynamicError::SerializationError(format!(
                "invalid signed default for primitive shape `{}`",
                primitive.as_str()
            ))),
        },
        (LiteralValue::UInt(value), FieldShape::Primitive(primitive)) => match primitive {
            FieldPrimitive::U8 => u8::try_from(*value)
                .map(DynamicValue::Uint8)
                .map_err(|error| DynamicError::SerializationError(error.to_string())),
            FieldPrimitive::U16 => u16::try_from(*value)
                .map(DynamicValue::Uint16)
                .map_err(|error| DynamicError::SerializationError(error.to_string())),
            FieldPrimitive::U32 => u32::try_from(*value)
                .map(DynamicValue::Uint32)
                .map_err(|error| DynamicError::SerializationError(error.to_string())),
            FieldPrimitive::U64 => Ok(DynamicValue::Uint64(*value)),
            _ => Err(DynamicError::SerializationError(format!(
                "invalid unsigned default for primitive shape `{}`",
                primitive.as_str()
            ))),
        },
        (LiteralValue::Float32(value), FieldShape::Primitive(FieldPrimitive::F32)) => {
            Ok(DynamicValue::Float32(*value))
        }
        (LiteralValue::Float64(value), FieldShape::Primitive(FieldPrimitive::F64)) => {
            Ok(DynamicValue::Float64(*value))
        }
        (LiteralValue::String(value), FieldShape::String | FieldShape::BoundedString { .. }) => {
            Ok(DynamicValue::String(value.clone()))
        }
        _ => Err(DynamicError::SerializationError(
            "default does not match runtime field shape".to_string(),
        )),
    }
}

fn parse_type_name(value: &str) -> Result<TypeName, DynamicError> {
    TypeName::new(value).map_err(|error| DynamicError::SerializationError(error.to_string()))
}

fn synthetic_enum_type_name(type_name: &str) -> String {
    match type_name.rsplit_once("::") {
        Some((prefix, name)) => format!("{prefix}::{name}__enum"),
        None => format!("{type_name}__enum"),
    }
}

fn logical_enum_type_name(type_name: &TypeName) -> String {
    type_name
        .as_str()
        .strip_suffix("__enum")
        .unwrap_or(type_name.as_str())
        .to_string()
}
