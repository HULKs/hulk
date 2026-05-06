use std::collections::BTreeMap;
use std::sync::Arc;

use ros_z_schema::{
    EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, NamedTypeDef, PrimitiveTypeDef,
    RootTypeName, SchemaBundle, SchemaDefinitions, SequenceLengthDef, StructDef, TypeDef, TypeName,
};

use super::DynamicError;
use super::schema::{
    FieldSchema, PrimitiveType, RuntimeDynamicEnumPayload, RuntimeDynamicEnumVariant, Schema,
    SequenceLength, TypeShape,
};
use crate::entity::SchemaHash;

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
) -> Result<SchemaHash, DynamicError> {
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
    Ok(FieldDef::new(field.name.clone(), shape))
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
                    })
                })
                .collect::<Result<Vec<_>, DynamicError>>()?,
        )),
    }
}
