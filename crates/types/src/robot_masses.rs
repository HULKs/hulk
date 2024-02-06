use coordinate_systems::Framed;
use nalgebra::{point, Point3};

use crate::coordinate_systems::{
    Head, LeftAnkle, LeftElbow, LeftFoot, LeftForearm, LeftHip, LeftPelvis, LeftShoulder,
    LeftThigh, LeftTibia, LeftUpperArm, LeftWrist, Neck, RightAnkle, RightElbow, RightFoot,
    RightForearm, RightHip, RightPelvis, RightShoulder, RightThigh, RightTibia, RightUpperArm,
    RightWrist, Torso,
};

#[derive(Debug)]
pub struct RobotMass<Frame> {
    pub mass: f32,
    pub center: Framed<Frame, Point3<f32>>,
}

pub const TORSO: RobotMass<Torso> = RobotMass {
    mass: 1.0496,
    center: Framed::new(point![0.0, 0.0, 0.0]),
};
// head
pub const NECK: RobotMass<Neck> = RobotMass {
    mass: 0.07842,
    center: Framed::new(point![-0.00001, 0.0, -0.02742]),
};
pub const HEAD: RobotMass<Head> = RobotMass {
    mass: 0.65937,
    center: Framed::new(point![0.00109, 0.00146, 0.05719]),
};
// left arm
pub const LEFT_SHOULDER: RobotMass<LeftShoulder> = RobotMass {
    mass: 0.09304,
    center: Framed::new(point![-0.00165, -0.02663, 0.00014]),
};
pub const LEFT_UPPER_ARM: RobotMass<LeftUpperArm> = RobotMass {
    mass: 0.15777,
    center: Framed::new(point![0.02455, 0.00563, 0.0033]),
};
pub const LEFT_ELBOW: RobotMass<LeftElbow> = RobotMass {
    mass: 0.06483,
    center: Framed::new(point![-0.02744, 0.0, -0.00014]),
};
pub const LEFT_FOREARM: RobotMass<LeftForearm> = RobotMass {
    mass: 0.07761,
    center: Framed::new(point![0.02556, 0.00281, 0.00076]),
};
pub const LEFT_WRIST: RobotMass<LeftWrist> = RobotMass {
    mass: 0.18533,
    center: Framed::new(point![0.03434, -0.00088, 0.00308]),
};
// right arm
pub const RIGHT_SHOULDER: RobotMass<RightShoulder> = RobotMass {
    mass: 0.09304,
    center: Framed::new(point![-0.00165, 0.02663, 0.00014]),
};
pub const RIGHT_UPPER_ARM: RobotMass<RightUpperArm> = RobotMass {
    mass: 0.15777,
    center: Framed::new(point![0.02455, -0.00563, 0.0033]),
};
pub const RIGHT_ELBOW: RobotMass<RightElbow> = RobotMass {
    mass: 0.06483,
    center: Framed::new(point![-0.02744, 0.0, -0.00014]),
};
pub const RIGHT_FOREARM: RobotMass<RightForearm> = RobotMass {
    mass: 0.07761,
    center: Framed::new(point![0.02556, -0.00281, 0.00076]),
};
pub const RIGHT_WRIST: RobotMass<RightWrist> = RobotMass {
    mass: 0.18533,
    center: Framed::new(point![0.03434, 0.00088, 0.00308]),
};
// left leg
pub const LEFT_PELVIS: RobotMass<LeftPelvis> = RobotMass {
    mass: 0.06981,
    center: Framed::new(point![-0.00781, -0.01114, 0.02661]),
};
pub const LEFT_HIP: RobotMass<LeftHip> = RobotMass {
    mass: 0.14053,
    center: Framed::new(point![-0.01549, 0.00029, -0.00515]),
};
pub const LEFT_THIGH: RobotMass<LeftThigh> = RobotMass {
    mass: 0.38968,
    center: Framed::new(point![0.00138, 0.00221, -0.05373]),
};
pub const LEFT_TIBIA: RobotMass<LeftTibia> = RobotMass {
    mass: 0.30142,
    center: Framed::new(point![0.00453, 0.00225, -0.04936]),
};
pub const LEFT_ANKLE: RobotMass<LeftAnkle> = RobotMass {
    mass: 0.13416,
    center: Framed::new(point![0.00045, 0.00029, 0.00685]),
};
pub const LEFT_FOOT: RobotMass<LeftFoot> = RobotMass {
    mass: 0.17184,
    center: Framed::new(point![0.02542, 0.0033, -0.03239]),
};
// right leg
pub const RIGHT_PELVIS: RobotMass<RightPelvis> = RobotMass {
    mass: 0.06981,
    center: Framed::new(point![-0.00781, 0.01114, 0.02661]),
};
pub const RIGHT_HIP: RobotMass<RightHip> = RobotMass {
    mass: 0.14053,
    center: Framed::new(point![-0.01549, -0.00029, -0.00515]),
};
pub const RIGHT_THIGH: RobotMass<RightThigh> = RobotMass {
    mass: 0.38968,
    center: Framed::new(point![0.00138, -0.00221, -0.05373]),
};
pub const RIGHT_TIBIA: RobotMass<RightTibia> = RobotMass {
    mass: 0.30142,
    center: Framed::new(point![0.00453, -0.00225, -0.04936]),
};
pub const RIGHT_ANKLE: RobotMass<RightAnkle> = RobotMass {
    mass: 0.13416,
    center: Framed::new(point![0.00045, -0.00029, 0.00685]),
};
pub const RIGHT_FOOT: RobotMass<RightFoot> = RobotMass {
    mass: 0.17184,
    center: Framed::new(point![0.02542, -0.0033, -0.03239]),
};

pub const TOTAL_MASS: f32 = TORSO.mass
    + NECK.mass
    + HEAD.mass
    + LEFT_SHOULDER.mass
    + LEFT_UPPER_ARM.mass
    + LEFT_ELBOW.mass
    + LEFT_FOREARM.mass
    + LEFT_WRIST.mass
    + RIGHT_SHOULDER.mass
    + RIGHT_UPPER_ARM.mass
    + RIGHT_ELBOW.mass
    + RIGHT_FOREARM.mass
    + RIGHT_WRIST.mass
    + LEFT_PELVIS.mass
    + LEFT_HIP.mass
    + LEFT_THIGH.mass
    + LEFT_TIBIA.mass
    + LEFT_ANKLE.mass
    + LEFT_FOOT.mass
    + RIGHT_PELVIS.mass
    + RIGHT_HIP.mass
    + RIGHT_THIGH.mass
    + RIGHT_TIBIA.mass
    + RIGHT_ANKLE.mass
    + RIGHT_FOOT.mass;
