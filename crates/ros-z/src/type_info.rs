use crate::dynamic::FieldType;
use crate::entity::TypeInfo;

fn field_type_generic_arg_name(field_type: &FieldType) -> String {
    match field_type {
        FieldType::Bool => "bool".to_string(),
        FieldType::Int8 => "i8".to_string(),
        FieldType::Int16 => "i16".to_string(),
        FieldType::Int32 => "i32".to_string(),
        FieldType::Int64 => "i64".to_string(),
        FieldType::Uint8 => "u8".to_string(),
        FieldType::Uint16 => "u16".to_string(),
        FieldType::Uint32 => "u32".to_string(),
        FieldType::Uint64 => "u64".to_string(),
        FieldType::Float32 => "f32".to_string(),
        FieldType::Float64 => "f64".to_string(),
        FieldType::String => "string".to_string(),
        FieldType::BoundedString(capacity) => format!("string_{}", capacity),
        FieldType::Message(schema) => schema.type_name_str().to_string(),
        FieldType::Optional(inner) => {
            format!("option_{}", field_type_generic_arg_name(inner.as_ref()))
        }
        FieldType::Enum(schema) => schema.type_name.clone(),
        FieldType::Array(inner, len) => {
            format!(
                "array_{}_{}",
                len,
                field_type_generic_arg_name(inner.as_ref())
            )
        }
        FieldType::Sequence(inner) => {
            format!("vec_{}", field_type_generic_arg_name(inner.as_ref()))
        }
        FieldType::BoundedSequence(inner, max) => {
            format!(
                "vec_{}_{}",
                max,
                field_type_generic_arg_name(inner.as_ref())
            )
        }
        FieldType::Map(key, value) => format!(
            "map_{}_{}",
            field_type_generic_arg_name(key.as_ref()),
            field_type_generic_arg_name(value.as_ref())
        ),
    }
}

pub trait FieldTypeInfo {
    fn field_type() -> crate::dynamic::FieldType;

    fn generic_arg_name() -> String {
        field_type_generic_arg_name(&Self::field_type())
    }
}

impl<T> FieldTypeInfo for T
where
    T: crate::msg::Message,
{
    fn field_type() -> crate::dynamic::FieldType {
        <T as crate::msg::Message>::field_type()
    }

    fn generic_arg_name() -> String {
        <T as crate::msg::Message>::type_name().to_string()
    }
}

macro_rules! impl_primitive_field_type_info {
    ($ty:ty, $field_type:expr, $generic_arg_name:expr) => {
        impl FieldTypeInfo for $ty {
            fn field_type() -> crate::dynamic::FieldType {
                $field_type
            }

            fn generic_arg_name() -> String {
                $generic_arg_name.to_string()
            }
        }
    };
}

impl_primitive_field_type_info!(bool, FieldType::Bool, "bool");
impl_primitive_field_type_info!(i8, FieldType::Int8, "i8");
impl_primitive_field_type_info!(u8, FieldType::Uint8, "u8");
impl_primitive_field_type_info!(i16, FieldType::Int16, "i16");
impl_primitive_field_type_info!(u16, FieldType::Uint16, "u16");
impl_primitive_field_type_info!(i32, FieldType::Int32, "i32");
impl_primitive_field_type_info!(u32, FieldType::Uint32, "u32");
impl_primitive_field_type_info!(i64, FieldType::Int64, "i64");
impl_primitive_field_type_info!(u64, FieldType::Uint64, "u64");
impl_primitive_field_type_info!(f32, FieldType::Float32, "f32");
impl_primitive_field_type_info!(f64, FieldType::Float64, "f64");
impl_primitive_field_type_info!(String, FieldType::String, "string");

impl<T: FieldTypeInfo> FieldTypeInfo for Vec<T> {
    fn field_type() -> crate::dynamic::FieldType {
        FieldType::Sequence(Box::new(T::field_type()))
    }
}

impl<T: FieldTypeInfo> FieldTypeInfo for Option<T> {
    fn field_type() -> crate::dynamic::FieldType {
        FieldType::Optional(Box::new(T::field_type()))
    }
}

impl<K: FieldTypeInfo, V: FieldTypeInfo> FieldTypeInfo for std::collections::HashMap<K, V> {
    fn field_type() -> crate::dynamic::FieldType {
        FieldType::Map(Box::new(K::field_type()), Box::new(V::field_type()))
    }
}

impl<K: FieldTypeInfo, V: FieldTypeInfo> FieldTypeInfo for std::collections::BTreeMap<K, V> {
    fn field_type() -> crate::dynamic::FieldType {
        FieldType::Map(Box::new(K::field_type()), Box::new(V::field_type()))
    }
}

impl<T: FieldTypeInfo, const N: usize> FieldTypeInfo for [T; N] {
    fn field_type() -> crate::dynamic::FieldType {
        FieldType::Array(Box::new(T::field_type()), N)
    }
}

/// Trait for ROS service types that provides service-level type information.
///
/// For services, the type name should be based on the service name (not Request/Response)
/// and the hash should be the composite service hash (not just request or response hash).
///
/// The service hash in ROS2 is computed from a composite type that includes:
/// - request_message (the Request type)
/// - response_message (the Response type)
/// - event_message (a virtual Event type containing ServiceEventInfo, request[], and response[])
///
pub trait ServiceTypeInfo {
    /// Returns the service type info (type name and hash for the service).
    fn service_type_info() -> TypeInfo;
}

/// Trait for ROS action types that provides action-level type information.
///
/// For actions, the type name should be based on the action name and the hash should be
/// the composite action hash.
///
pub trait ActionTypeInfo {
    /// Returns the action type info (type name and hash for the action).
    fn action_type_info() -> TypeInfo;
}
