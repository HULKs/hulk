use core::panic;
use std::{collections::VecDeque, time::SystemTime};

use color_eyre::Result;
use hardware::PathsInterface;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::{deserialize_not_implemented, MainOutput};
use tflite::{ops::builtin::BuiltinOpResolver, FlatBufferModel, InterpreterBuilder};
use types::fall_state::{Direction, FallState, Kind, Side};

#[derive(Deserialize, Serialize)]
pub struct FallStateDetection {
    last_fall_state: FallState,
    #[serde(skip, default = "deserialize_not_implemented")]
    model: FlatBufferModel,
    data: VecDeque<f32>,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    // _difference_to_sitting_threshold:
    //     Parameter<f32, "fall_state_estimation.difference_to_sitting_threshold">,
    // _falling_angle_threshold_forward:
    //     Parameter<Vector2<Robot>, "fall_state_estimation.falling_angle_threshold_forward">,
    // _falling_angle_threshold_forward_with_catching_steps: Parameter<
    //     Vector2<Robot>,
    //     "fall_state_estimation.falling_angle_threshold_forward_with_catching_steps",
    // >,
    // _falling_timeout: Parameter<Duration, "fall_state_estimation.falling_timeout">,
    // _gravitational_acceleration_threshold:
    //     Parameter<f32, "fall_state_estimation.gravitational_acceleration_threshold">,
    // _gravitational_force_sitting:
    //     Parameter<Vector3<Robot>, "fall_state_estimation.gravitational_force_sitting">,
    // _gravity_acceleration: Parameter<f32, "physical_constants.gravity_acceleration">,
    // _sitting_pose: Parameter<Joints<f32>, "fall_state_estimation.sitting_pose">,
    // _catching_steps_enabled: Parameter<bool, "walking_engine.catching_steps.enabled">,
    //
    // _robot_orientation: RequiredInput<Option<Orientation3<Field>>, "robot_orientation?">,
    // _sensor_data: Input<SensorData, "sensor_data">,
    // _cycle_time: Input<CycleTime, "cycle_time">,
    // _has_ground_contact: Input<bool, "has_ground_contact">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub fall_state_tinyml: MainOutput<FallState>,
}

impl FallStateDetection {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;

        let model_path = neural_network_folder.join("fall_detection.tflite");

        let model = FlatBufferModel::build_from_file(model_path)?;

        Ok(Self {
            last_fall_state: Default::default(),
            model,
            data: VecDeque::new(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        // TODO: hadle primary state unstiff

        // let cycle_start = context.cycle_time.start_time;
        // let inertial_measurement_unit = context.sensor_data.inertial_measurement_unit;
        // let (roll, pitch, _) = context.robot_orientation.inner.euler_angles();

        let resolver = BuiltinOpResolver::default();

        let builder = InterpreterBuilder::new(&self.model, &resolver)?;
        let mut interpreter = builder.build()?;
        interpreter.set_num_threads(1);

        interpreter.allocate_tensors()?;

        let inputs = interpreter.inputs().to_vec();
        assert_eq!(inputs.len(), 1);

        let input_index = inputs[0];

        let outputs = interpreter.outputs().to_vec();
        assert_eq!(outputs.len(), 1);

        let output_index = outputs[0];

        let input_tensor = interpreter.tensor_info(input_index).unwrap();
        // dbg!(&input_tensor.dims);

        self.data.push_back(0.0);
        self.data.push_back(0.0);
        self.data.push_back(0.0);
        self.data.push_back(0.0);

        let max_size = input_tensor.dims[1] * input_tensor.dims[2];
        while self.data.len() > max_size {
            self.data.pop_front();
        }

        if self.data.len() == max_size {
            interpreter
                .tensor_data_mut(input_index)
                .unwrap()
                .copy_from_slice(self.data.make_contiguous());
        }

        interpreter.invoke()?;

        let output: &[f32] = interpreter.tensor_data(output_index)?;

        let guess = output
            .iter()
            .enumerate()
            .max_by(|x, y| x.1.total_cmp(y.1))
            .unwrap()
            .0;

        let fall_state = match guess {
            0 => FallState::Upright,
            1 => FallState::Falling {
                start_time: SystemTime::UNIX_EPOCH,
                direction: Direction::Forward { side: Side::Left },
            },
            2 => FallState::Fallen {
                kind: Kind::FacingUp,
            },
            _ => panic!(),
        };

        Ok(MainOutputs {
            fall_state_tinyml: fall_state.into(),
        })
    }
}
