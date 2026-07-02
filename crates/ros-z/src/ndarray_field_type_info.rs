use ros_z_schema::{PrimitiveTypeDef, SchemaError, SequenceLengthDef, TypeDef};

use crate::{Message, SerdeCdrCodec};

macro_rules! impl_dynamic_sequence_message {
    ($ty:ty, $type_name:literal, $primitive:ident) => {
        impl Message for $ty {
            type Codec = SerdeCdrCodec<Self>;

            fn type_name() -> String {
                $type_name.to_string()
            }
        }

        impl crate::schema::MessageSchema for $ty {
            fn build_schema(
                _builder: &mut crate::schema::SchemaBuilder,
            ) -> Result<TypeDef, SchemaError> {
                Ok(TypeDef::Sequence {
                    element: Box::new(TypeDef::Primitive(PrimitiveTypeDef::$primitive)),
                    length: SequenceLengthDef::Dynamic,
                })
            }
        }
    };
}

impl_dynamic_sequence_message!(ndarray::Array2<f32>, "ndarray::Array2<f32>", F32);
