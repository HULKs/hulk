use booster_sdk::types::RobotMode;
use hardware::{
    ActuatorInterface, HighLevelInterface, LightControlInterface, LowCommandInterface,
    LowStateInterface, MotionRuntimeInterface, NetworkInterface, PathsInterface,
    RecordingInterface, SimulatorInterface, SpeakerInterface, TimeInterface, VisualKickInterface,
};

use color_eyre::eyre::Result;

use hula_types::hardware::Paths;
use kinematics::joints::{Joints, head::HeadJoints};
use types::{
    audio::SpeakerRequest,
    led::Leds,
    messages::{IncomingMessage, OutgoingMessage},
    motion_runtime::MotionRuntime,
    step::Step,
};

pub trait HardwareInterface:
    ActuatorInterface
    + LowCommandInterface
    + VisualKickInterface
    + LowStateInterface
    + NetworkInterface
    + PathsInterface
    + RecordingInterface
    + SpeakerInterface
    + TimeInterface
    + SimulatorInterface
    + HighLevelInterface
    + MotionRuntimeInterface
    + LightControlInterface
{
}

pub struct ExtractorHardwareInterface;

/// `write_to_actuators` is a noop during replay
impl ActuatorInterface for ExtractorHardwareInterface {
    fn write_to_actuators(
        &self,
        _positions: Joints<f32>,
        _stiffnesses: Joints<f32>,
        _leds: Leds,
    ) -> Result<()> {
        Ok(())
    }
}

impl LowCommandInterface for ExtractorHardwareInterface {
    fn write_low_command(&self, _low_command: booster::LowCommand) -> Result<()> {
        unimplemented!()
    }
}

impl VisualKickInterface for ExtractorHardwareInterface {
    fn write_visual_kick(&self, _kick: booster::Kick) -> Result<()> {
        unimplemented!()
    }
}

impl LowStateInterface for ExtractorHardwareInterface {
    fn read_low_state(&self) -> Result<booster::LowState> {
        unimplemented!()
    }
}

/// `read_from_network` is only executed in setup nodes, which are not executed during replay
/// `write_to_network` is a noop during replay
impl NetworkInterface for ExtractorHardwareInterface {
    fn read_from_network(&self) -> Result<IncomingMessage> {
        panic!("failed to read from network during replay")
    }

    fn write_to_network(&self, _message: OutgoingMessage) -> Result<()> {
        Ok(())
    }
}

/// recording is not supported for replaying
impl RecordingInterface for ExtractorHardwareInterface {
    fn should_record(&self) -> bool {
        false
    }

    fn set_whether_to_record(&self, _enable: bool) {}
}

/// imagine does not produce speaker outputs
impl SpeakerInterface for ExtractorHardwareInterface {
    fn write_to_speakers(&self, _request: SpeakerRequest) {}
}

impl PathsInterface for ExtractorHardwareInterface {
    fn get_paths(&self) -> Paths {
        Paths {
            motions: "etc/motions".into(),
            neural_networks: "etc/neural_networks".into(),
            sounds: "etc/sounds".into(),
            cache: ".cache".into(),
        }
    }
}

impl TimeInterface for ExtractorHardwareInterface {
    fn get_now(&self) -> std::time::SystemTime {
        unimplemented!()
    }
}

impl SimulatorInterface for ExtractorHardwareInterface {
    fn is_simulation(&self) -> Result<bool> {
        unimplemented!()
    }
}

impl HighLevelInterface for ExtractorHardwareInterface {
    fn change_mode(&self, _mode: RobotMode) -> Result<()> {
        unimplemented!()
    }

    fn get_mode(&self) -> Result<RobotMode> {
        unimplemented!()
    }

    fn move_robot(&self, _step: Step) -> Result<()> {
        unimplemented!()
    }

    fn rotate_head(&self, _head_joints: HeadJoints<f32>) -> Result<()> {
        unimplemented!()
    }

    fn rotate_head_with_direction(&self, _head_joints: HeadJoints<i32>) -> Result<()> {
        unimplemented!()
    }

    fn lie_down(&self) -> Result<()> {
        unimplemented!()
    }

    fn get_up(&self) -> Result<()> {
        unimplemented!()
    }

    fn get_up_with_mode(&self, _mode: RobotMode) -> Result<()> {
        unimplemented!()
    }

    fn enter_wbc_gait(&self) -> Result<()> {
        unimplemented!()
    }

    fn exit_wbc_gait(&self) -> Result<()> {
        unimplemented!()
    }

    fn visual_kick(&self, _start: bool) -> Result<()> {
        unimplemented!()
    }

    fn reset_odometer(&self) -> Result<()> {
        Ok(())
    }
}

impl MotionRuntimeInterface for ExtractorHardwareInterface {
    fn get_motion_runtime_type(&self) -> Result<MotionRuntime> {
        unimplemented!()
    }
}

impl LightControlInterface for ExtractorHardwareInterface {
    fn set_led_color(
        &self,
        _light_control_parameter: booster_sdk::client::light_control::SetLedLightColorParameter,
    ) -> Result<()> {
        Ok(())
    }

    fn stop_led_control(&self) -> Result<()> {
        Ok(())
    }
}

impl HardwareInterface for ExtractorHardwareInterface {}
