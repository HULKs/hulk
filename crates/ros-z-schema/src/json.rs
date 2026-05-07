use std::fmt::Write;

use crate::{
    ActionDef, EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, PrimitiveTypeDef, SchemaBundle,
    SchemaError, SequenceLengthDef, ServiceDef, StructDef, TypeDef, TypeDefinition,
    TypeDefinitions, TypeName,
};

/// Writes a deterministic compact JSON projection for schema values.
pub trait JsonEncode {
    /// Appends the JSON encoding to `out`.
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError>;
}

/// Serializes a value into its JSON representation.
pub fn to_json<T: JsonEncode>(value: &T) -> Result<String, SchemaError> {
    let mut out = String::new();
    value.write_json(&mut out)?;
    Ok(out)
}

impl JsonEncode for SchemaBundle {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        out.push('{');
        write_json_string("definitions", out);
        out.push(':');
        self.definitions.write_json(out)?;
        out.push(',');
        write_json_string("root", out);
        out.push(':');
        self.root.write_json(out)?;
        out.push('}');
        Ok(())
    }
}

impl JsonEncode for TypeDefinitions {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        out.push('{');
        let mut first = true;
        for (type_name, definition) in self.iter() {
            if !first {
                out.push(',');
            }
            first = false;
            type_name.write_json(out)?;
            out.push(':');
            definition.write_json(out)?;
        }
        out.push('}');
        Ok(())
    }
}

impl JsonEncode for TypeName {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        write_json_string(self.as_str(), out);
        Ok(())
    }
}

impl JsonEncode for TypeDefinition {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        out.push('{');
        match self {
            Self::Struct(definition) => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("struct", out);
                out.push(',');
                write_json_string("fields", out);
                out.push(':');
                definition.write_json(out)?;
            }
            Self::Enum(definition) => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("enum", out);
                out.push(',');
                write_json_string("variants", out);
                out.push(':');
                definition.write_json(out)?;
            }
        }
        out.push('}');
        Ok(())
    }
}

impl JsonEncode for TypeDef {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        out.push('{');
        match self {
            Self::Primitive(primitive) => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("primitive", out);
                out.push(',');
                write_json_string("name", out);
                out.push(':');
                primitive.write_json(out)?;
            }
            Self::String => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("string", out);
            }
            Self::Named(type_name) => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("named", out);
                out.push(',');
                write_json_string("type", out);
                out.push(':');
                type_name.write_json(out)?;
            }
            Self::Optional(element) => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("optional", out);
                out.push(',');
                write_json_string("element", out);
                out.push(':');
                element.write_json(out)?;
            }
            Self::Sequence { element, length } => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("sequence", out);
                out.push(',');
                write_json_string("length", out);
                out.push(':');
                length.write_json(out)?;
                out.push(',');
                write_json_string("element", out);
                out.push(':');
                element.write_json(out)?;
            }
            Self::Map { key, value } => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("map", out);
                out.push(',');
                write_json_string("key", out);
                out.push(':');
                key.write_json(out)?;
                out.push(',');
                write_json_string("value", out);
                out.push(':');
                value.write_json(out)?;
            }
        }
        out.push('}');
        Ok(())
    }
}

impl JsonEncode for PrimitiveTypeDef {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        write_json_string(self.as_str(), out);
        Ok(())
    }
}

impl JsonEncode for SequenceLengthDef {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        out.push('{');
        match self {
            Self::Dynamic => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("dynamic", out);
            }
            Self::Fixed(value) => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("fixed", out);
                out.push(',');
                write_json_string("value", out);
                out.push(':');
                let _ = write!(out, "{value}");
            }
        }
        out.push('}');
        Ok(())
    }
}

impl JsonEncode for StructDef {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        out.push('[');
        let mut first = true;
        for field in &self.fields {
            if !first {
                out.push(',');
            }
            first = false;
            field.write_json(out)?;
        }
        out.push(']');
        Ok(())
    }
}

impl JsonEncode for EnumDef {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        out.push('[');
        let mut first = true;
        for variant in &self.variants {
            if !first {
                out.push(',');
            }
            first = false;
            variant.write_json(out)?;
        }
        out.push(']');
        Ok(())
    }
}

impl JsonEncode for EnumVariantDef {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        out.push('{');
        write_json_string("name", out);
        out.push(':');
        write_json_string(&self.name, out);
        out.push(',');
        write_json_string("payload", out);
        out.push(':');
        self.payload.write_json(out)?;
        out.push('}');
        Ok(())
    }
}

impl JsonEncode for EnumPayloadDef {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        out.push('{');
        match self {
            Self::Unit => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("unit", out);
            }
            Self::Newtype(shape) => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("newtype", out);
                out.push(',');
                write_json_string("shape", out);
                out.push(':');
                shape.write_json(out)?;
            }
            Self::Tuple(shapes) => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("tuple", out);
                out.push(',');
                write_json_string("shapes", out);
                out.push(':');
                out.push('[');
                let mut first = true;
                for shape in shapes {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    shape.write_json(out)?;
                }
                out.push(']');
            }
            Self::Struct(fields) => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("struct", out);
                out.push(',');
                write_json_string("fields", out);
                out.push(':');
                out.push('[');
                let mut first = true;
                for field in fields {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    field.write_json(out)?;
                }
                out.push(']');
            }
        }
        out.push('}');
        Ok(())
    }
}

impl JsonEncode for FieldDef {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        out.push('{');
        write_json_string("name", out);
        out.push(':');
        write_json_string(&self.name, out);
        out.push(',');
        write_json_string("shape", out);
        out.push(':');
        self.shape.write_json(out)?;

        out.push('}');
        Ok(())
    }
}

impl JsonEncode for ServiceDef {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        out.push('{');
        write_json_string("request", out);
        out.push(':');
        self.request.write_json(out)?;
        out.push(',');
        write_json_string("response", out);
        out.push(':');
        self.response.write_json(out)?;
        out.push(',');
        write_json_string("type_name", out);
        out.push(':');
        self.type_name.write_json(out)?;
        out.push('}');
        Ok(())
    }
}

impl JsonEncode for ActionDef {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        out.push('{');
        write_json_string("feedback", out);
        out.push(':');
        self.feedback.write_json(out)?;
        out.push(',');
        write_json_string("goal", out);
        out.push(':');
        self.goal.write_json(out)?;
        out.push(',');
        write_json_string("result", out);
        out.push(':');
        self.result.write_json(out)?;
        out.push(',');
        write_json_string("type_name", out);
        out.push(':');
        self.type_name.write_json(out)?;
        out.push('}');
        Ok(())
    }
}

fn write_json_string(value: &str, out: &mut String) {
    out.push('"');

    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch <= '\u{1F}' => {
                let _ = write!(out, "\\u{:04x}", u32::from(ch));
            }
            ch => out.push(ch),
        }
    }

    out.push('"');
}
