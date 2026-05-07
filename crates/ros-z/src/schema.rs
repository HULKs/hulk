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

pub struct StructSchemaBuilder<'a> {
    builder: &'a mut SchemaBuilder,
    fields: Vec<FieldDef>,
}

pub struct EnumSchemaBuilder<'a> {
    builder: &'a mut SchemaBuilder,
    variants: Vec<EnumVariantDef>,
}

pub struct TupleVariantSchemaBuilder<'a> {
    builder: &'a mut SchemaBuilder,
    elements: Vec<TypeDef>,
}

impl SchemaBuilder {
    pub fn new() -> Self {
        Self {
            definitions: BTreeMap::new(),
            in_progress: BTreeMap::new(),
            failed: false,
        }
    }

    pub fn define_message_struct<T>(
        &mut self,
        build_fields: impl FnOnce(&mut StructSchemaBuilder<'_>) -> Result<(), SchemaError>,
    ) -> Result<TypeDef, SchemaError>
    where
        T: Message,
    {
        let name = TypeName::new(T::type_name())?;
        self.define_struct(name, build_fields)
    }

    pub fn define_message_enum<T>(
        &mut self,
        build_variants: impl FnOnce(&mut EnumSchemaBuilder<'_>) -> Result<(), SchemaError>,
    ) -> Result<TypeDef, SchemaError>
    where
        T: Message,
    {
        let name = TypeName::new(T::type_name())?;
        self.define_enum(name, build_variants)
    }

    pub fn define_struct(
        &mut self,
        name: TypeName,
        build_fields: impl FnOnce(&mut StructSchemaBuilder<'_>) -> Result<(), SchemaError>,
    ) -> Result<TypeDef, SchemaError> {
        self.define_named(name, DefinitionKind::Struct, |builder| {
            let mut fields = StructSchemaBuilder {
                builder,
                fields: Vec::new(),
            };
            build_fields(&mut fields)?;
            Ok(TypeDefinition::Struct(StructDef {
                fields: fields.fields,
            }))
        })
    }

    pub fn define_enum(
        &mut self,
        name: TypeName,
        build_variants: impl FnOnce(&mut EnumSchemaBuilder<'_>) -> Result<(), SchemaError>,
    ) -> Result<TypeDef, SchemaError> {
        self.define_named(name, DefinitionKind::Enum, |builder| {
            let mut variants = EnumSchemaBuilder {
                builder,
                variants: Vec::new(),
            };
            build_variants(&mut variants)?;
            Ok(TypeDefinition::Enum(EnumDef {
                variants: variants.variants,
            }))
        })
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

    fn define_named(
        &mut self,
        name: TypeName,
        attempted: DefinitionKind,
        build_definition: impl FnOnce(&mut Self) -> Result<TypeDefinition, SchemaError>,
    ) -> Result<TypeDef, SchemaError> {
        self.ensure_usable()?;
        if let Some(definition) = self.definitions.get(&name) {
            let existing = definition.kind();
            if existing == attempted {
                return Ok(TypeDef::Named(name));
            }
            return Err(SchemaError::DefinitionKindConflict {
                name,
                existing,
                attempted,
            });
        }
        if let Some(existing) = self.in_progress.get(&name).copied() {
            if existing == attempted {
                return Ok(TypeDef::Named(name));
            }
            return Err(SchemaError::DefinitionKindConflict {
                name,
                existing,
                attempted,
            });
        }

        self.in_progress.insert(name.clone(), attempted);
        let definition = match build_definition(self) {
            Ok(definition) => definition,
            Err(error) => {
                self.in_progress.remove(&name);
                self.failed = true;
                return Err(error);
            }
        };
        self.in_progress.remove(&name);
        self.definitions.insert(name.clone(), definition);
        Ok(TypeDef::Named(name))
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

impl StructSchemaBuilder<'_> {
    pub fn field<T>(&mut self, name: impl Into<String>) -> Result<(), SchemaError>
    where
        T: MessageSchema,
    {
        let shape = T::build_schema(self.builder)?;
        self.field_with_shape(name, shape);
        Ok(())
    }

    pub fn field_with_shape(&mut self, name: impl Into<String>, shape: impl Into<TypeDef>) {
        self.fields.push(FieldDef::new(name, shape));
    }

    pub fn shape<T>(&mut self) -> Result<TypeDef, SchemaError>
    where
        T: MessageSchema,
    {
        T::build_schema(self.builder)
    }
}

impl EnumSchemaBuilder<'_> {
    pub fn unit(&mut self, name: impl Into<String>) {
        self.variants
            .push(EnumVariantDef::new(name, EnumPayloadDef::Unit));
    }

    pub fn newtype<T>(&mut self, name: impl Into<String>) -> Result<(), SchemaError>
    where
        T: MessageSchema,
    {
        let shape = T::build_schema(self.builder)?;
        self.newtype_with_shape(name, shape);
        Ok(())
    }

    pub fn newtype_with_shape(&mut self, name: impl Into<String>, shape: impl Into<TypeDef>) {
        self.variants.push(EnumVariantDef::new(
            name,
            EnumPayloadDef::Newtype(shape.into()),
        ));
    }

    pub fn shape<T>(&mut self) -> Result<TypeDef, SchemaError>
    where
        T: MessageSchema,
    {
        T::build_schema(self.builder)
    }

    pub fn tuple(
        &mut self,
        name: impl Into<String>,
        build_fields: impl FnOnce(&mut TupleVariantSchemaBuilder<'_>) -> Result<(), SchemaError>,
    ) -> Result<(), SchemaError> {
        let mut fields = TupleVariantSchemaBuilder {
            builder: &mut *self.builder,
            elements: Vec::new(),
        };
        build_fields(&mut fields)?;
        self.variants.push(EnumVariantDef::new(
            name,
            EnumPayloadDef::Tuple(fields.elements),
        ));
        Ok(())
    }

    pub fn struct_variant(
        &mut self,
        name: impl Into<String>,
        build_fields: impl FnOnce(&mut StructSchemaBuilder<'_>) -> Result<(), SchemaError>,
    ) -> Result<(), SchemaError> {
        let mut fields = StructSchemaBuilder {
            builder: &mut *self.builder,
            fields: Vec::new(),
        };
        build_fields(&mut fields)?;
        self.variants.push(EnumVariantDef::new(
            name,
            EnumPayloadDef::Struct(fields.fields),
        ));
        Ok(())
    }
}

impl TupleVariantSchemaBuilder<'_> {
    pub fn element<T>(&mut self) -> Result<(), SchemaError>
    where
        T: MessageSchema,
    {
        let shape = T::build_schema(self.builder)?;
        self.element_with_shape(shape);
        Ok(())
    }

    pub fn element_with_shape(&mut self, shape: impl Into<TypeDef>) {
        self.elements.push(shape.into());
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
        builder.define_struct(name, |fields| {
            fields.field::<TypeDef>("root")?;
            fields.field::<TypeDefinitions>("definitions")?;
            Ok(())
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
        builder.define_enum(name, |variants| {
            variants.newtype::<PrimitiveTypeDef>("Primitive")?;
            variants.unit("String");
            variants.newtype::<TypeName>("Named")?;
            variants.newtype::<TypeDef>("Optional")?;
            variants.struct_variant("Sequence", |fields| {
                fields.field::<TypeDef>("element")?;
                fields.field::<SequenceLengthDef>("length")?;
                Ok(())
            })?;
            variants.struct_variant("Map", |fields| {
                fields.field::<TypeDef>("key")?;
                fields.field::<TypeDef>("value")?;
                Ok(())
            })?;
            Ok(())
        })
    }
}

impl MessageSchema for SequenceLengthDef {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::SequenceLengthDef")?;
        builder.define_enum(name, |variants| {
            variants.unit("Dynamic");
            variants.newtype::<usize>("Fixed")?;
            Ok(())
        })
    }
}

impl MessageSchema for TypeDefinition {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::TypeDefinition")?;
        builder.define_enum(name, |variants| {
            variants.newtype::<StructDef>("Struct")?;
            variants.newtype::<EnumDef>("Enum")?;
            Ok(())
        })
    }
}

impl MessageSchema for StructDef {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::StructDef")?;
        builder.define_struct(name, |fields| {
            fields.field::<Vec<FieldDef>>("fields")?;
            Ok(())
        })
    }
}

impl MessageSchema for EnumDef {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::EnumDef")?;
        builder.define_struct(name, |fields| {
            fields.field::<Vec<EnumVariantDef>>("variants")?;
            Ok(())
        })
    }
}

impl MessageSchema for EnumVariantDef {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::EnumVariantDef")?;
        builder.define_struct(name, |fields| {
            fields.field::<String>("name")?;
            fields.field::<EnumPayloadDef>("payload")?;
            Ok(())
        })
    }
}

impl MessageSchema for EnumPayloadDef {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::EnumPayloadDef")?;
        builder.define_enum(name, |variants| {
            variants.unit("Unit");
            variants.newtype::<TypeDef>("Newtype")?;
            variants.newtype::<Vec<TypeDef>>("Tuple")?;
            variants.newtype::<Vec<FieldDef>>("Struct")?;
            Ok(())
        })
    }
}

impl MessageSchema for FieldDef {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new("ros_z_schema::FieldDef")?;
        builder.define_struct(name, |fields| {
            fields.field::<String>("name")?;
            fields.field::<TypeDef>("shape")?;
            Ok(())
        })
    }
}
