use approx::AbsDiffEq;
use approx_derive::RelativeEq;
use serde::{Deserialize, Serialize};

macro_rules! generate_coordinate_system {
    ($($i:ident),*) => {
        $(
            #[derive(
                Clone,
                Copy,
                Debug,
                Default,
                Deserialize,
                Eq,
                Hash,
                PartialEq,
                Serialize,
                RelativeEq,
            )]
            pub struct $i;

            impl AbsDiffEq for $i {
                type Epsilon = f32;

                fn default_epsilon() -> Self::Epsilon {
                    Self::Epsilon::default_epsilon()
                }

                fn abs_diff_eq(&self, _other: &Self, _epsilon: Self::Epsilon) -> bool {
                    true
                }
            }
        )*
    }
}

generate_coordinate_system!(
    Robot,
    Ground,
    Field,
    Camera,
    Pixel,
    Head,
    Neck,
    Torso,
    LeftShoulder,
    LeftUpperArm,
    LeftElbow,
    LeftForearm,
    LeftWrist,
    RightShoulder,
    RightUpperArm,
    RightElbow,
    RightForearm,
    RightWrist,
    LeftPelvis,
    LeftHip,
    LeftThigh,
    LeftTibia,
    LeftAnkle,
    LeftFoot,
    LeftSole,
    RightPelvis,
    RightHip,
    RightThigh,
    RightTibia,
    RightAnkle,
    RightFoot,
    RightSole
);
