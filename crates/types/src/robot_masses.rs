use nalgebra::{point, Point3};

#[derive(Debug)]
pub struct RobotMass {
    pub mass: f32,
    pub center: Point3<f32>,
}

impl RobotMass {
    pub const TORSO: RobotMass = RobotMass {
        mass: 1.0496,
        center: point![0.0, 0.0, 0.0],
    };
    // head
    pub const NECK: RobotMass = RobotMass {
        mass: 0.07842,
        center: point![-0.00001, 0.0, -0.02742],
    };
    pub const HEAD: RobotMass = RobotMass {
        mass: 0.65937,
        center: point![0.00109, 0.00146, 0.05719],
    };
    // left arm
    pub const LEFT_SHOULDER: RobotMass = RobotMass {
        mass: 0.09304,
        center: point![-0.00165, -0.02663, 0.00014],
    };
    pub const LEFT_UPPER_ARM: RobotMass = RobotMass {
        mass: 0.15777,
        center: point![0.02455, 0.00563, 0.0033],
    };
    pub const LEFT_ELBOW: RobotMass = RobotMass {
        mass: 0.06483,
        center: point![-0.02744, 0.0, -0.00014],
    };
    pub const LEFT_FOREARM: RobotMass = RobotMass {
        mass: 0.07761,
        center: point![0.02556, 0.00281, 0.00076],
    };
    pub const LEFT_WRIST: RobotMass = RobotMass {
        mass: 0.18533,
        center: point![0.03434, -0.00088, 0.00308],
    };
    // right arm
    pub const RIGHT_SHOULDER: RobotMass = RobotMass {
        mass: 0.09304,
        center: point![-0.00165, 0.02663, 0.00014],
    };
    pub const RIGHT_UPPER_ARM: RobotMass = RobotMass {
        mass: 0.15777,
        center: point![0.02455, -0.00563, 0.0033],
    };
    pub const RIGHT_ELBOW: RobotMass = RobotMass {
        mass: 0.06483,
        center: point![-0.02744, 0.0, -0.00014],
    };
    pub const RIGHT_FOREARM: RobotMass = RobotMass {
        mass: 0.07761,
        center: point![0.02556, -0.00281, 0.00076],
    };
    pub const RIGHT_WRIST: RobotMass = RobotMass {
        mass: 0.18533,
        center: point![0.03434, 0.00088, 0.00308],
    };
    // left leg
    pub const LEFT_PELVIS: RobotMass = RobotMass {
        mass: 0.06981,
        center: point![-0.00781, -0.01114, 0.02661],
    };
    pub const LEFT_HIP: RobotMass = RobotMass {
        mass: 0.14053,
        center: point![-0.01549, 0.00029, -0.00515],
    };
    pub const LEFT_THIGH: RobotMass = RobotMass {
        mass: 0.38968,
        center: point![0.00138, 0.00221, -0.05373],
    };
    pub const LEFT_TIBIA: RobotMass = RobotMass {
        mass: 0.30142,
        center: point![0.00453, 0.00225, -0.04936],
    };
    pub const LEFT_ANKLE: RobotMass = RobotMass {
        mass: 0.13416,
        center: point![0.00045, 0.00029, 0.00685],
    };
    pub const LEFT_FOOT: RobotMass = RobotMass {
        mass: 0.17184,
        center: point![0.02542, 0.0033, -0.03239],
    };
    // right leg
    pub const RIGHT_PELVIS: RobotMass = RobotMass {
        mass: 0.06981,
        center: point![-0.00781, 0.01114, 0.02661],
    };
    pub const RIGHT_HIP: RobotMass = RobotMass {
        mass: 0.14053,
        center: point![-0.01549, -0.00029, -0.00515],
    };
    pub const RIGHT_THIGH: RobotMass = RobotMass {
        mass: 0.38968,
        center: point![0.00138, -0.00221, -0.05373],
    };
    pub const RIGHT_TIBIA: RobotMass = RobotMass {
        mass: 0.30142,
        center: point![0.00453, -0.00225, -0.04936],
    };
    pub const RIGHT_ANKLE: RobotMass = RobotMass {
        mass: 0.13416,
        center: point![0.00045, -0.00029, 0.00685],
    };
    pub const RIGHT_FOOT: RobotMass = RobotMass {
        mass: 0.17184,
        center: point![0.02542, -0.0033, -0.03239],
    };

    pub const TOTAL_MASS: f32 = Self::TORSO.mass
        + Self::NECK.mass
        + Self::HEAD.mass
        + Self::LEFT_SHOULDER.mass
        + Self::LEFT_UPPER_ARM.mass
        + Self::LEFT_ELBOW.mass
        + Self::LEFT_FOREARM.mass
        + Self::LEFT_WRIST.mass
        + Self::RIGHT_SHOULDER.mass
        + Self::RIGHT_UPPER_ARM.mass
        + Self::RIGHT_ELBOW.mass
        + Self::RIGHT_FOREARM.mass
        + Self::RIGHT_WRIST.mass
        + Self::LEFT_PELVIS.mass
        + Self::LEFT_HIP.mass
        + Self::LEFT_THIGH.mass
        + Self::LEFT_TIBIA.mass
        + Self::LEFT_ANKLE.mass
        + Self::LEFT_FOOT.mass
        + Self::RIGHT_PELVIS.mass
        + Self::RIGHT_HIP.mass
        + Self::RIGHT_THIGH.mass
        + Self::RIGHT_TIBIA.mass
        + Self::RIGHT_ANKLE.mass
        + Self::RIGHT_FOOT.mass;
}
