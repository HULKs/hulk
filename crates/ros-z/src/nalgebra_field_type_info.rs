use nalgebra::{
    Isometry2, Isometry3, Matrix2, Matrix3, Matrix4, Point2, Point3, Rotation2, Rotation3,
    Translation2, Translation3, UnitComplex, UnitQuaternion, Vector2, Vector3, Vector4,
};
use ros_z_schema::{PrimitiveTypeDef, SchemaError, SequenceLengthDef, TypeDef, TypeName};

use crate::{Message, SerdeCdrCodec};

macro_rules! impl_fixed_sequence_message {
    ($ty:ty, $type_name:literal, $primitive:ident, $len:expr) => {
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
                    length: SequenceLengthDef::Fixed($len),
                })
            }
        }
    };
}

impl_fixed_sequence_message!(Vector2<f32>, "nalgebra::Vector2<f32>", F32, 2);
impl_fixed_sequence_message!(Vector2<f64>, "nalgebra::Vector2<f64>", F64, 2);
impl_fixed_sequence_message!(Vector3<f32>, "nalgebra::Vector3<f32>", F32, 3);
impl_fixed_sequence_message!(Vector3<f64>, "nalgebra::Vector3<f64>", F64, 3);
impl_fixed_sequence_message!(Vector4<f32>, "nalgebra::Vector4<f32>", F32, 4);
impl_fixed_sequence_message!(Vector4<f64>, "nalgebra::Vector4<f64>", F64, 4);
impl_fixed_sequence_message!(Point2<f32>, "nalgebra::Point2<f32>", F32, 2);
impl_fixed_sequence_message!(Point2<f64>, "nalgebra::Point2<f64>", F64, 2);
impl_fixed_sequence_message!(Point2<u16>, "nalgebra::Point2<u16>", U16, 2);
impl_fixed_sequence_message!(Point3<f32>, "nalgebra::Point3<f32>", F32, 3);
impl_fixed_sequence_message!(Point3<f64>, "nalgebra::Point3<f64>", F64, 3);
impl_fixed_sequence_message!(Translation2<f32>, "nalgebra::Translation2<f32>", F32, 2);
impl_fixed_sequence_message!(Translation2<f64>, "nalgebra::Translation2<f64>", F64, 2);
impl_fixed_sequence_message!(Translation3<f32>, "nalgebra::Translation3<f32>", F32, 3);
impl_fixed_sequence_message!(Translation3<f64>, "nalgebra::Translation3<f64>", F64, 3);
impl_fixed_sequence_message!(Matrix2<f32>, "nalgebra::Matrix2<f32>", F32, 4);
impl_fixed_sequence_message!(Matrix2<f64>, "nalgebra::Matrix2<f64>", F64, 4);
impl_fixed_sequence_message!(Matrix3<f32>, "nalgebra::Matrix3<f32>", F32, 9);
impl_fixed_sequence_message!(Matrix3<f64>, "nalgebra::Matrix3<f64>", F64, 9);
impl_fixed_sequence_message!(Matrix4<f32>, "nalgebra::Matrix4<f32>", F32, 16);
impl_fixed_sequence_message!(Matrix4<f64>, "nalgebra::Matrix4<f64>", F64, 16);
impl_fixed_sequence_message!(Rotation2<f32>, "nalgebra::Rotation2<f32>", F32, 4);
impl_fixed_sequence_message!(Rotation2<f64>, "nalgebra::Rotation2<f64>", F64, 4);
impl_fixed_sequence_message!(Rotation3<f32>, "nalgebra::Rotation3<f32>", F32, 9);
impl_fixed_sequence_message!(Rotation3<f64>, "nalgebra::Rotation3<f64>", F64, 9);
impl_fixed_sequence_message!(UnitComplex<f32>, "nalgebra::UnitComplex<f32>", F32, 2);
impl_fixed_sequence_message!(UnitComplex<f64>, "nalgebra::UnitComplex<f64>", F64, 2);
impl_fixed_sequence_message!(UnitQuaternion<f32>, "nalgebra::UnitQuaternion<f32>", F32, 4);
impl_fixed_sequence_message!(UnitQuaternion<f64>, "nalgebra::UnitQuaternion<f64>", F64, 4);

macro_rules! impl_isometry_message {
    ($ty:ty, $type_name:literal, $rotation:ty, $translation:ty) => {
        impl Message for $ty {
            type Codec = SerdeCdrCodec<Self>;

            fn type_name() -> String {
                $type_name.to_string()
            }
        }

        impl crate::schema::MessageSchema for $ty {
            fn build_schema(
                builder: &mut crate::schema::SchemaBuilder,
            ) -> Result<TypeDef, SchemaError> {
                let name = TypeName::new($type_name)?;
                builder.define_struct(name, |fields| {
                    fields.field::<$rotation>("rotation")?;
                    fields.field::<$translation>("translation")?;
                    Ok(())
                })
            }
        }
    };
}

impl_isometry_message!(
    Isometry2<f32>,
    "nalgebra::Isometry2<f32>",
    UnitComplex<f32>,
    Translation2<f32>
);
impl_isometry_message!(
    Isometry2<f64>,
    "nalgebra::Isometry2<f64>",
    UnitComplex<f64>,
    Translation2<f64>
);
impl_isometry_message!(
    Isometry3<f32>,
    "nalgebra::Isometry3<f32>",
    UnitQuaternion<f32>,
    Translation3<f32>
);
impl_isometry_message!(
    Isometry3<f64>,
    "nalgebra::Isometry3<f64>",
    UnitQuaternion<f64>,
    Translation3<f64>
);
