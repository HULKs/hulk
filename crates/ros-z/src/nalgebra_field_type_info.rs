use std::sync::Arc;

use nalgebra::{
    Isometry2, Isometry3, Matrix2, Matrix3, Point2, Point3, Rotation2, Rotation3, Translation2,
    Translation3, UnitComplex, UnitQuaternion, Vector2, Vector3,
};
use ros_z_schema::TypeName;

use crate::{
    Message, SerdeCdrCodec,
    dynamic::{PrimitiveType, RuntimeFieldSchema, Schema, SequenceLength, TypeShape},
};

macro_rules! impl_fixed_sequence_message {
    ($ty:ty, $type_name:literal, $primitive:ident, $len:expr) => {
        impl Message for $ty {
            type Codec = SerdeCdrCodec<Self>;

            fn type_name() -> &'static str {
                $type_name
            }

            fn schema() -> Schema {
                Arc::new(TypeShape::Sequence {
                    element: Arc::new(TypeShape::Primitive(PrimitiveType::$primitive)),
                    length: SequenceLength::Fixed($len),
                })
            }
        }
    };
}

impl_fixed_sequence_message!(Vector2<f32>, "nalgebra::Vector2<f32>", F32, 2);
impl_fixed_sequence_message!(Vector2<f64>, "nalgebra::Vector2<f64>", F64, 2);
impl_fixed_sequence_message!(Vector3<f32>, "nalgebra::Vector3<f32>", F32, 3);
impl_fixed_sequence_message!(Vector3<f64>, "nalgebra::Vector3<f64>", F64, 3);
impl_fixed_sequence_message!(Point2<f32>, "nalgebra::Point2<f32>", F32, 2);
impl_fixed_sequence_message!(Point2<f64>, "nalgebra::Point2<f64>", F64, 2);
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
impl_fixed_sequence_message!(Rotation2<f32>, "nalgebra::Rotation2<f32>", F32, 4);
impl_fixed_sequence_message!(Rotation2<f64>, "nalgebra::Rotation2<f64>", F64, 4);
impl_fixed_sequence_message!(Rotation3<f32>, "nalgebra::Rotation3<f32>", F32, 9);
impl_fixed_sequence_message!(Rotation3<f64>, "nalgebra::Rotation3<f64>", F64, 9);
impl_fixed_sequence_message!(UnitComplex<f32>, "nalgebra::UnitComplex<f32>", F32, 2);
impl_fixed_sequence_message!(UnitComplex<f64>, "nalgebra::UnitComplex<f64>", F64, 2);
impl_fixed_sequence_message!(UnitQuaternion<f32>, "nalgebra::UnitQuaternion<f32>", F32, 4);
impl_fixed_sequence_message!(UnitQuaternion<f64>, "nalgebra::UnitQuaternion<f64>", F64, 4);

fn isometry_schema(type_name: &str, rotation: Schema, translation: Schema) -> Schema {
    Arc::new(TypeShape::Struct {
        name: TypeName::new(type_name.to_string()).expect("valid nalgebra type name"),
        fields: vec![
            RuntimeFieldSchema::new("rotation", rotation),
            RuntimeFieldSchema::new("translation", translation),
        ],
    })
}

macro_rules! impl_isometry_message {
    ($ty:ty, $type_name:literal, $rotation:ty, $translation:ty) => {
        impl Message for $ty {
            type Codec = SerdeCdrCodec<Self>;

            fn type_name() -> &'static str {
                $type_name
            }

            fn schema() -> Schema {
                isometry_schema(
                    $type_name,
                    <$rotation as Message>::schema(),
                    <$translation as Message>::schema(),
                )
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
