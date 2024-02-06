use approx_derive::{AbsDiffEq, RelativeEq};
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
                AbsDiffEq,
            )]
            #[abs_diff_eq(epsilon = "f32")]
            pub struct $i;
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
