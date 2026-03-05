use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::mode::Mode;

#[allow(unused)]
#[derive(Clone, Debug)]
enum ApiId {
    ChangeMode = 2000,
    Move = 2001,
    RotateHead = 2004,
    WaveHand = 2005,
    RotateHeadWithDirection = 2006,
    LieDown = 2007,
    GetUp = 2008,
    MoveHandEndEffector = 2009,
    ControlGripper = 2010,
    GetFrameTransform = 2011,
    SwitchHandEndEffectorControlMode = 2012,
    ControlDexterousHand = 2013,
    Handshake = 2015,
    Dance = 2016,
    GetMode = 2017,
    GetStatus = 2018,
    PushUp = 2019,
    PlaySound = 2020,
    StopSound = 2021,
    GetRobotInfo = 2022,
    StopHandEndEffector = 2023,
    Shoot = 2024,
    GetUpWithMode = 2025,
    ZeroTorqueDrag = 2026,
    RecordTrajectory = 2027,
    ReplayTrajectory = 2028,
    WholeBodyDance = 2029,
    UpperBodyCustomControl = 2030,
    ResetOdometry = 2031,
    LoadCustomTrainedTraj = 2032,
    ActivateCustomTrainedTraj = 2033,
    UnloadCustomTrainedTraj = 2034,
    EnterWBCGait = 2035,
    ExitWBCGait = 2036,
}

#[repr(C)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request {
    uuid: String,
    header: String,
    body: String,
}

impl Request {
    fn new(id: ApiId, body: impl Into<String>) -> Self {
        let uuid = Uuid::new_v4().to_string();
        let header = format!("{{\"api_id\":{id}}}", id = id as usize);
        let body = body.into();

        Self { uuid, header, body }
    }

    pub fn change_mode(mode: Mode) -> Self {
        Self::new(
            ApiId::ChangeMode,
            format!("{{\"mode\":{mode}}}", mode = mode as usize),
        )
    }

    pub fn enter_wbc_gait() -> Self {
        Self::new(ApiId::EnterWBCGait, "")
    }

    pub fn exit_wbc_gait() -> Self {
        Self::new(ApiId::ExitWBCGait, "")
    }

    pub fn get_up() -> Self {
        Self::new(ApiId::GetUp, "")
    }
}
