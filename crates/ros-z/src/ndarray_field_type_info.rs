use ros_z_schema::{PrimitiveTypeDef, SchemaError, SequenceLengthDef, TypeDef};

use crate::{CdrCodec, Message};
use ros_z_cdr::{CdrBuffer, CdrReader, CdrWriter, LittleEndian};

macro_rules! impl_dynamic_sequence_message {
    ($ty:ty, $type_name:literal, $primitive:ident) => {
        impl Message for $ty {
            type Codec = CdrCodec<Self>;
            fn encode_cdr<B: CdrBuffer>(
                &self,
                writer: &mut CdrWriter<'_, LittleEndian, B>,
            ) -> ros_z_cdr::Result<()> {
                crate::serde_bridge::encode_serde(self, writer)
            }

            fn decode_cdr(reader: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
                crate::serde_bridge::decode_serde(reader)
            }

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
