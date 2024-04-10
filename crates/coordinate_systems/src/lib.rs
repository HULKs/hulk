use approx_derive::{AbsDiffEq, RelativeEq};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

macro_rules! generate_coordinate_system {
    ($($(#[$doc:meta])* $i:ident),* $(,)?) => {
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
                SerializeHierarchy
            )]
            #[abs_diff_eq(epsilon = "f32")]
            $(#[$doc])*
            pub struct $i;
        )*
    }
}

generate_coordinate_system!(
    /// 3D coordinate system centered on the robot.
    ///
    /// Origin: hip of the robot
    /// X axis pointing forward
    Robot,
    /// 2D coordinate system centered on the robot.
    ///
    /// Origin: center between [LeftSole] and [RightSole], projected onto the ground.
    /// X axis pointing forward
    Ground,
    /// 2D coordinate system centered on the field,
    ///
    /// Origin: center of the field
    /// X axis pointing towards the opponent goal
    Field,
    /// 3D Intrinsic coordinate system of the camera.
    ///
    /// Origin: center of the camera model
    /// X axis pointing right, Y axis pointing down, Z axis pointing forward
    NormalizedDeviceCoordinates,
    /// 3D coordinate system centered on the camera
    ///
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/video_naov6.html)
    Camera,
    /// 2D Coordinate system of the camera image.
    ///
    /// Origin: top left corner of the image
    /// X axis points right
    /// Y axis points down
    Pixel,
    /// 2D Coordinate system of the camera image.
    /// Same as [Pixel] but the dimensions are normalized to (0.0, 1.0).
    NormalizedPixel,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#head)
    Head,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#neck)
    Neck,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#torso)
    Torso,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#left-shoulder)
    LeftShoulder,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#left-biceps)
    LeftUpperArm,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#left-elbow)
    LeftElbow,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#left-forearm)
    LeftForearm,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#left-hand)
    LeftWrist,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#right-shoulder)
    RightShoulder,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#right-biceps)
    RightUpperArm,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#right-elbow)
    RightElbow,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#right-forearm)
    RightForearm,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#right-hand)
    RightWrist,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#left-pelvis)
    LeftPelvis,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#left-hip)
    LeftHip,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#left-thigh)
    LeftThigh,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#left-tibia)
    LeftTibia,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#left-ankle)
    LeftAnkle,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#left-foot)
    LeftFoot,
    /// Same as [LeftFoot] but shifted down to the sole.
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#left-foot)
    LeftSole,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#right-pelvis)
    RightPelvis,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#right-hip)
    RightHip,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#right-thigh)
    RightThigh,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#right-tibia)
    RightTibia,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#right-ankle)
    RightAnkle,
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#right-foot)
    RightFoot,
    /// Same as [RightFoot] but shifted down to the sole.
    /// See [official documentation](http://doc.aldebaran.com/2-8/family/nao_technical/masses_naov6.html#right-foot)
    RightSole,
);
