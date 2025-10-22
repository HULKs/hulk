use coordinate_systems::{
    Head, LeftAnkle, LeftFoot, LeftForearm, LeftHip, LeftInnerShoulder, LeftOuterShoulder,
    LeftPelvis, LeftThigh, LeftTibia, LeftUpperArm, Neck, RightAnkle, RightFoot, RightForearm,
    RightHip, RightInnerShoulder, RightOuterShoulder, RightPelvis, RightThigh, RightTibia,
    RightUpperArm, Torso,
};
use linear_algebra::{point, Point3};

#[derive(Debug)]
pub struct RobotMass<Frame> {
    pub mass: f32,
    pub center: Point3<Frame>,
}

pub const TORSO: RobotMass<Torso> = RobotMass {
    mass: 6.3921,
    center: point![-0.0043392, -0.00065534, 0.065686],
};
// head
pub const NECK: RobotMass<Neck> = RobotMass {
    mass: 0.29729,
    center: point![-0.00069503, -0.00038527, 0.031688],
};
pub const HEAD: RobotMass<Head> = RobotMass {
    mass: 0.61387,
    center: point![0.011042, -0.00092871, 0.080698],
};
// left arm
pub const LEFT_INNER_SHOULDER: RobotMass<LeftInnerShoulder> = RobotMass {
    mass: 0.45628,
    center: point![-0.002701, 0.062383, -0.011965],
};
pub const LEFT_OUTER_SHOULDER: RobotMass<LeftOuterShoulder> = RobotMass {
    mass: 0.084303,
    center: point![0.01729, 0.018221, -4.329e-05],
};
pub const LEFT_UPPER_ARM: RobotMass<LeftUpperArm> = RobotMass {
    mass: 0.76208,
    center: point![-0.0003979, 0.072617, -0.00066218],
};
pub const LEFT_FOREARM: RobotMass<LeftForearm> = RobotMass {
    mass: 0.16796,
    center: point![-0.0015751, 0.084637, 0.0087033],
};
// right arm
pub const RIGHT_INNER_SHOULDER: RobotMass<RightInnerShoulder> = RobotMass {
    mass: 0.45669,
    center: point![-0.0026302, -0.06251, -0.01199],
};
pub const RIGHT_OUTER_SHOULDER: RobotMass<RightOuterShoulder> = RobotMass {
    mass: 0.083353,
    center: point![0.017229, -0.018428, 4.3784e-05],
};
pub const RIGHT_UPPER_ARM: RobotMass<RightUpperArm> = RobotMass {
    mass: 0.76234,
    center: point![-0.00039919, -0.072601, -0.00065629],
};
pub const RIGHT_FOREARM: RobotMass<RightForearm> = RobotMass {
    mass: 0.16828,
    center: point![-0.0015177, -0.084536, 0.0086733],
};

// left leg
pub const LEFT_PELVIS: RobotMass<LeftPelvis> = RobotMass {
    mass: 0.64068,
    center: point![-0.010093, -0.0019934, -0.024713],
};
pub const LEFT_HIP: RobotMass<LeftHip> = RobotMass {
    mass: 0.127825,
    center: point![0.023585, 4.9e-05, -0.024996],
};
pub const LEFT_THIGH: RobotMass<LeftThigh> = RobotMass {
    mass: 1.5591,
    center: point![-0.0084744, -0.0040494, -0.087906],
};
pub const LEFT_TIBIA: RobotMass<LeftTibia> = RobotMass {
    mass: 1.48333,
    center: point![-0.00081083, 0.0031459, -0.10922],
};
pub const LEFT_ANKLE: RobotMass<LeftAnkle> = RobotMass {
    mass: 0.038836,
    center: point![0.0, 0.0, 0.0],
};
pub const LEFT_FOOT: RobotMass<LeftFoot> = RobotMass {
    mass: 0.38305,
    center: point![7.3589e-05, -4.1141e-06, -0.0075032],
};
// right leg
pub const RIGHT_PELVIS: RobotMass<RightPelvis> = RobotMass {
    mass: 0.6413,
    center: point![-0.010065, 0.0020086, -0.024739],
};
pub const RIGHT_HIP: RobotMass<RightHip> = RobotMass {
    mass: 0.127825,
    center: point![0.023585, 4.9e-05, -0.024996],
};
pub const RIGHT_THIGH: RobotMass<RightThigh> = RobotMass {
    mass: 1.5592,
    center: point![-0.008475, 0.0040392, -0.087906],
};
pub const RIGHT_TIBIA: RobotMass<RightTibia> = RobotMass {
    mass: 1.48334,
    center: point![-0.000805, -0.003146, -0.109215],
};
pub const RIGHT_ANKLE: RobotMass<RightAnkle> = RobotMass {
    mass: 0.038836,
    center: point![0.0, 0.0, 0.0],
};
pub const RIGHT_FOOT: RobotMass<RightFoot> = RobotMass {
    mass: 0.38637,
    center: point![-0.000178, -2e-06, -0.007291],
};

pub const TOTAL_MASS: f32 = TORSO.mass
    + NECK.mass
    + HEAD.mass
    + LEFT_INNER_SHOULDER.mass
    + LEFT_OUTER_SHOULDER.mass
    + LEFT_UPPER_ARM.mass
    + LEFT_FOREARM.mass
    + RIGHT_INNER_SHOULDER.mass
    + RIGHT_OUTER_SHOULDER.mass
    + RIGHT_UPPER_ARM.mass
    + RIGHT_FOREARM.mass
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
