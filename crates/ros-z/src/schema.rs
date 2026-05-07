use std::collections::BTreeMap;

use ros_z_schema::{
    DefinitionKind, EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, PrimitiveTypeDef,
    SchemaBundle, SchemaError, SequenceLengthDef, StructDef, TypeDef, TypeDefinition,
    TypeDefinitions, TypeName,
};

use crate::Message;

pub trait MessageSchema {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError>;
}

pub struct SchemaBuilder {
    definitions: BTreeMap<TypeName, TypeDefinition>,
    in_progress: BTreeMap<TypeName, DefinitionKind>,
    failed: bool,
}

impl SchemaBuilder {
    pub fn new() -> Self {
        Self {
            definitions: BTreeMap::new(),
            in_progress: BTreeMap::new(),
            failed: false,
        }
    }

    pub fn define_struct(
        &mut self,
        name: TypeName,
        build_fields: impl FnOnce(&mut Self) -> Result<Vec<FieldDef>, SchemaError>,
    ) -> Result<TypeDef, SchemaError> {
        self.ensure_usable()?;
        if let Some(definition) = self.definitions.get(&name) {
            return match definition {
                TypeDefinition::Struct(_) => Ok(TypeDef::Named(name)),
                TypeDefinition::Enum(_) => Err(SchemaError::DefinitionKindConflict {
                    name,
                    existing: DefinitionKind::Enum,
                    attempted: DefinitionKind::Struct,
                }),
            };
        }
        if let Some(kind) = self.in_progress.get(&name).copied() {
            return match kind {
                DefinitionKind::Struct => Ok(TypeDef::Named(name)),
                DefinitionKind::Enum => Err(SchemaError::DefinitionKindConflict {
                    name,
                    existing: DefinitionKind::Enum,
                    attempted: DefinitionKind::Struct,
                }),
            };
        }

        self.in_progress
            .insert(name.clone(), DefinitionKind::Struct);
        let fields = match build_fields(self) {
            Ok(fields) => fields,
            Err(error) => {
                self.in_progress.remove(&name);
                self.failed = true;
                return Err(error);
            }
        };
        self.in_progress.remove(&name);
        self.definitions
            .insert(name.clone(), TypeDefinition::Struct(StructDef { fields }));
        Ok(TypeDef::Named(name))
    }

    pub fn define_enum(
        &mut self,
        name: TypeName,
        build_variants: impl FnOnce(&mut Self) -> Result<Vec<EnumVariantDef>, SchemaError>,
    ) -> Result<TypeDef, SchemaError> {
        self.ensure_usable()?;
        if let Some(definition) = self.definitions.get(&name) {
            return match definition {
                TypeDefinition::Enum(_) => Ok(TypeDef::Named(name)),
                TypeDefinition::Struct(_) => Err(SchemaError::DefinitionKindConflict {
                    name,
                    existing: DefinitionKind::Struct,
                    attempted: DefinitionKind::Enum,
                }),
            };
        }
        if let Some(kind) = self.in_progress.get(&name).copied() {
            return match kind {
                DefinitionKind::Enum => Ok(TypeDef::Named(name)),
                DefinitionKind::Struct => Err(SchemaError::DefinitionKindConflict {
                    name,
                    existing: DefinitionKind::Struct,
                    attempted: DefinitionKind::Enum,
                }),
            };
        }

        self.in_progress.insert(name.clone(), DefinitionKind::Enum);
        let variants = match build_variants(self) {
            Ok(variants) => variants,
            Err(error) => {
                self.in_progress.remove(&name);
                self.failed = true;
                return Err(error);
            }
        };
        self.in_progress.remove(&name);
        self.definitions
            .insert(name.clone(), TypeDefinition::Enum(EnumDef { variants }));
        Ok(TypeDef::Named(name))
    }

    pub fn finish(self, root: TypeDef) -> Result<SchemaBundle, SchemaError> {
        if self.failed {
            return Err(SchemaError::BuilderFailed);
        }
        let bundle = SchemaBundle {
            root,
            definitions: TypeDefinitions::from(self.definitions),
        };
        bundle.validate()?;
        Ok(bundle)
    }

    fn ensure_usable(&self) -> Result<(), SchemaError> {
        if self.failed {
            Err(SchemaError::BuilderFailed)
        } else {
            Ok(())
        }
    }
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub fn schema_for<T>() -> Result<SchemaBundle, SchemaError>
where
    T: Message + MessageSchema,
{
    let mut builder = SchemaBuilder::new();
    let root = <T as MessageSchema>::build_schema(&mut builder)?;
    builder.finish(root)
}

impl MessageSchema for TypeName {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        String::build_schema(builder)
    }
}

impl MessageSchema for PrimitiveTypeDef {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        String::build_schema(builder)
    }
}

impl MessageSchema for SchemaBundle {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::SchemaBundle")?;
        builder.define_struct(name, |builder| {
            Ok(vec![
                FieldDef::new("root", TypeDef::build_schema(builder)?),
                FieldDef::new("definitions", TypeDefinitions::build_schema(builder)?),
            ])
        })
    }
}

impl MessageSchema for TypeDefinitions {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        Ok(TypeDef::Map {
            key: Box::new(TypeDef::String),
            value: Box::new(TypeDefinition::build_schema(builder)?),
        })
    }
}

impl MessageSchema for TypeDef {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::TypeDef")?;
        builder.define_enum(name, |builder| {
            Ok(vec![
                EnumVariantDef::new(
                    "Primitive",
                    EnumPayloadDef::Newtype(PrimitiveTypeDef::build_schema(builder)?),
                ),
                EnumVariantDef::new("String", EnumPayloadDef::Unit),
                EnumVariantDef::new(
                    "Named",
                    EnumPayloadDef::Newtype(TypeName::build_schema(builder)?),
                ),
                EnumVariantDef::new(
                    "Optional",
                    EnumPayloadDef::Newtype(TypeDef::build_schema(builder)?),
                ),
                EnumVariantDef::new(
                    "Sequence",
                    EnumPayloadDef::Struct(vec![
                        FieldDef::new("element", TypeDef::build_schema(builder)?),
                        FieldDef::new("length", SequenceLengthDef::build_schema(builder)?),
                    ]),
                ),
                EnumVariantDef::new(
                    "Map",
                    EnumPayloadDef::Struct(vec![
                        FieldDef::new("key", TypeDef::build_schema(builder)?),
                        FieldDef::new("value", TypeDef::build_schema(builder)?),
                    ]),
                ),
            ])
        })
    }
}

impl MessageSchema for SequenceLengthDef {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::SequenceLengthDef")?;
        builder.define_enum(name, |builder| {
            Ok(vec![
                EnumVariantDef::new("Dynamic", EnumPayloadDef::Unit),
                EnumVariantDef::new(
                    "Fixed",
                    EnumPayloadDef::Newtype(usize::build_schema(builder)?),
                ),
            ])
        })
    }
}

impl MessageSchema for TypeDefinition {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::TypeDefinition")?;
        builder.define_enum(name, |builder| {
            Ok(vec![
                EnumVariantDef::new(
                    "Struct",
                    EnumPayloadDef::Newtype(StructDef::build_schema(builder)?),
                ),
                EnumVariantDef::new(
                    "Enum",
                    EnumPayloadDef::Newtype(EnumDef::build_schema(builder)?),
                ),
            ])
        })
    }
}

impl MessageSchema for StructDef {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::StructDef")?;
        builder.define_struct(name, |builder| {
            Ok(vec![FieldDef::new(
                "fields",
                Vec::<FieldDef>::build_schema(builder)?,
            )])
        })
    }
}

impl MessageSchema for EnumDef {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::EnumDef")?;
        builder.define_struct(name, |builder| {
            Ok(vec![FieldDef::new(
                "variants",
                Vec::<EnumVariantDef>::build_schema(builder)?,
            )])
        })
    }
}

impl MessageSchema for EnumVariantDef {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::EnumVariantDef")?;
        builder.define_struct(name, |builder| {
            Ok(vec![
                FieldDef::new("name", String::build_schema(builder)?),
                FieldDef::new("payload", EnumPayloadDef::build_schema(builder)?),
            ])
        })
    }
}

impl MessageSchema for EnumPayloadDef {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::EnumPayloadDef")?;
        builder.define_enum(name, |builder| {
            Ok(vec![
                EnumVariantDef::new("Unit", EnumPayloadDef::Unit),
                EnumVariantDef::new(
                    "Newtype",
                    EnumPayloadDef::Newtype(TypeDef::build_schema(builder)?),
                ),
                EnumVariantDef::new(
                    "Tuple",
                    EnumPayloadDef::Newtype(Vec::<TypeDef>::build_schema(builder)?),
                ),
                EnumVariantDef::new(
                    "Struct",
                    EnumPayloadDef::Newtype(Vec::<FieldDef>::build_schema(builder)?),
                ),
            ])
        })
    }
}

impl MessageSchema for FieldDef {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::FieldDef")?;
        builder.define_struct(name, |builder| {
            Ok(vec![
                FieldDef::new("name", String::build_schema(builder)?),
                FieldDef::new("shape", TypeDef::build_schema(builder)?),
            ])
        })
    }
}
