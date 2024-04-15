use std::fmt::{self, Display, Formatter};

use enum_iterator::Sequence;

#[derive(Copy, Clone, Debug)]
pub enum SpeakerRequest {
    PlaySound { sound: Sound },
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Sequence)]
pub enum Sound {
    Ball,
    Bishop,
    CameraReset,
    CenterCircle,
    Corner,
    Defender,
    DefenderLeft,
    DefenderRight,
    Donk,
    Drift,
    FalsePositive,
    FalsePositiveDetected,
    Front,
    FrontLeft,
    FrontRight,
    GameControllerCollision,
    InvalidImage,
    Keeper,
    Left,
    LolaDesync,
    Ouch,
    PenaltyArea,
    PenaltySpot,
    Rear,
    RearLeft,
    RearRight,
    ReplacementKeeper,
    Right,
    SameNumberTuhhNao21,
    SameNumberTuhhNao22,
    SameNumberTuhhNao23,
    SameNumberTuhhNao24,
    SameNumberTuhhNao25,
    SameNumberTuhhNao26,
    SameNumberTuhhNao27,
    SameNumberTuhhNao28,
    SameNumberTuhhNao29,
    SameNumberTuhhNao30,
    SameNumberTuhhNao31,
    SameNumberTuhhNao32,
    SameNumberTuhhNao33,
    SameNumberTuhhNao34,
    SameNumberTuhhNao35,
    SameNumberTuhhNao36,
    SameNumberUnknownHULKDeviceEth,
    SameNumberUnknownHULKDeviceWifi,
    Sigh,
    Squat,
    Striker,
    Supporter,
    TJunction,
    UsbStickMissing,
    Weeeee,
}

impl Display for Sound {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
