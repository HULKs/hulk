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
        center: point![-0.0001, 0.0, -0.2742],
    };
    pub const HEAD: RobotMass = RobotMass {
        mass: 0.65937,
        center: point![0.0109, 0.0146, 0.5719],
    };
    // left arm
    pub const LEFT_SHOULDER: RobotMass = RobotMass {
        mass: 0.09304,
        center: point![-0.0165, -0.2663, 0.0014],
    };
    pub const LEFT_UPPER_ARM: RobotMass = RobotMass {
        mass: 0.15777,
        center: point![0.2455, 0.0563, 0.033],
    };
    pub const LEFT_ELBOW: RobotMass = RobotMass {
        mass: 0.06483,
        center: point![-0.2744, 0.0, -0.0014],
    };
    pub const LEFT_FOREARM: RobotMass = RobotMass {
        mass: 0.07761,
        center: point![0.2556, 0.0281, 0.0076],
    };
    pub const LEFT_WRIST: RobotMass = RobotMass {
        mass: 0.18533,
        center: point![0.3434, -0.0088, 0.0308],
    };
    // right arm
    pub const RIGHT_SHOULDER: RobotMass = RobotMass {
        mass: 0.09304,
        center: point![-0.0165, 0.2663, 0.0014],
    };
    pub const RIGHT_UPPER_ARM: RobotMass = RobotMass {
        mass: 0.15777,
        center: point![0.2455, -0.0563, 0.033],
    };
    pub const RIGHT_ELBOW: RobotMass = RobotMass {
        mass: 0.06483,
        center: point![-0.2744, 0.0, -0.0014],
    };
    pub const RIGHT_FOREARM: RobotMass = RobotMass {
        mass: 0.07761,
        center: point![0.2556, -0.0281, 0.0076],
    };
    pub const RIGHT_WRIST: RobotMass = RobotMass {
        mass: 0.18533,
        center: point![0.3434, 0.0088, 0.0308],
    };
    // left leg
    pub const LEFT_PELVIS: RobotMass = RobotMass {
        mass: 0.06981,
        center: point![-0.0781, -0.1114, 0.2661],
    };
    pub const LEFT_HIP: RobotMass = RobotMass {
        mass: 0.14053,
        center: point![-0.1549, 0.0029, -0.0515],
    };
    pub const LEFT_THIGH: RobotMass = RobotMass {
        mass: 0.38968,
        center: point![0.0138, 0.0221, -0.5373],
    };
    pub const LEFT_TIBIA: RobotMass = RobotMass {
        mass: 0.30142,
        center: point![0.0453, 0.0225, -0.4936],
    };
    pub const LEFT_ANKLE: RobotMass = RobotMass {
        mass: 0.13416,
        center: point![0.0045, 0.0029, 0.0685],
    };
    pub const LEFT_FOOT: RobotMass = RobotMass {
        mass: 0.17184,
        center: point![0.2542, 0.033, -0.3239],
    };
    // right leg
    pub const RIGHT_PELVIS: RobotMass = RobotMass {
        mass: 0.06981,
        center: point![-0.0781, 0.1114, 0.2661],
    };
    pub const RIGHT_HIP: RobotMass = RobotMass {
        mass: 0.14053,
        center: point![-0.1549, -0.0029, -0.0515],
    };
    pub const RIGHT_THIGH: RobotMass = RobotMass {
        mass: 0.38968,
        center: point![0.0138, -0.0221, -0.5373],
    };
    pub const RIGHT_TIBIA: RobotMass = RobotMass {
        mass: 0.30142,
        center: point![0.0453, -0.0225, -0.4936],
    };
    pub const RIGHT_ANKLE: RobotMass = RobotMass {
        mass: 0.13416,
        center: point![0.0045, -0.0029, 0.0685],
    };
    pub const RIGHT_FOOT: RobotMass = RobotMass {
        mass: 0.17184,
        center: point![0.2542, -0.033, -0.3239],
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
