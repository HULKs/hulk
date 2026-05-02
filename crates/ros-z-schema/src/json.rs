use std::fmt::Write;

use crate::{
    ActionDef, EnumDef, EnumPayloadDef, EnumVariantDef, FieldDef, LiteralValue, NamedTypeDef,
    PrimitiveTypeDef, SchemaBundle, SchemaDefinitions, SchemaError, SequenceLengthDef, ServiceDef,
    StructDef, TypeDef, TypeName,
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

impl JsonEncode for SchemaDefinitions {
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

impl JsonEncode for NamedTypeDef {
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
            Self::StructRef(type_name) => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("struct_ref", out);
                out.push(',');
                write_json_string("type", out);
                out.push(':');
                type_name.write_json(out)?;
            }
            Self::EnumRef(type_name) => {
                write_json_string("kind", out);
                out.push(':');
                write_json_string("enum_ref", out);
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

        if let Some(default) = &self.default {
            out.push(',');
            write_json_string("default", out);
            out.push(':');
            default.write_json(out)?;
        }

        out.push('}');
        Ok(())
    }
}

impl JsonEncode for LiteralValue {
    fn write_json(&self, out: &mut String) -> Result<(), SchemaError> {
        match self {
            Self::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Self::Int(value) => {
                let _ = write!(out, "{value}");
            }
            Self::UInt(value) => {
                let _ = write!(out, "{value}");
            }
            Self::Float32(value) => write_json_float32(*value, false, out)?,
            Self::Float64(value) => write_json_float64(*value, false, out)?,
            Self::String(value) => write_json_string(value, out),
            Self::BoolArray(values) => write_json_array(values, out, |value, out| {
                out.push_str(if *value { "true" } else { "false" });
                Ok(())
            })?,
            Self::IntArray(values) => write_json_array(values, out, |value, out| {
                let _ = write!(out, "{value}");
                Ok(())
            })?,
            Self::UIntArray(values) => write_json_array(values, out, |value, out| {
                let _ = write!(out, "{value}");
                Ok(())
            })?,
            Self::Float32Array(values) => write_json_array(values, out, |value, out| {
                write_json_float32(*value, true, out)
            })?,
            Self::Float64Array(values) => write_json_array(values, out, |value, out| {
                write_json_float64(*value, true, out)
            })?,
            Self::StringArray(values) => write_json_array(values, out, |value, out| {
                write_json_string(value, out);
                Ok(())
            })?,
        }
        Ok(())
    }
}

fn write_json_array<T>(
    values: &[T],
    out: &mut String,
    mut write_value: impl FnMut(&T, &mut String) -> Result<(), SchemaError>,
) -> Result<(), SchemaError> {
    out.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        write_value(value, out)?;
    }
    out.push(']');
    Ok(())
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

fn write_json_float32(value: f32, is_array: bool, out: &mut String) -> Result<(), SchemaError> {
    if !value.is_finite() {
        return Err(SchemaError::InvalidLiteralValue(if is_array {
            "non-finite float32[] literal".to_string()
        } else {
            "non-finite float32 literal".to_string()
        }));
    }

    let _ = write!(out, "{value}");
    Ok(())
}

fn write_json_float64(value: f64, is_array: bool, out: &mut String) -> Result<(), SchemaError> {
    if !value.is_finite() {
        return Err(SchemaError::InvalidLiteralValue(if is_array {
            "non-finite float64[] literal".to_string()
        } else {
            "non-finite float64 literal".to_string()
        }));
    }

    let _ = write!(out, "{value}");
    Ok(())
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
