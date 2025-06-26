use std::time::Duration;

use color_eyre::Result;
use coordinate_systems::{Field, Robot};
use hardware::PathsInterface;
use linear_algebra::{Orientation3, Vector2, Vector3};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use tflite::{ops::builtin::BuiltinOpResolver, FlatBufferModel, InterpreterBuilder};
use types::{
    cycle_time::CycleTime, fall_state::FallState, joints::Joints, sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct FallStateDetection {
    last_fall_state: FallState,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    _difference_to_sitting_threshold:
        Parameter<f32, "fall_state_estimation.difference_to_sitting_threshold">,
    _falling_angle_threshold_forward:
        Parameter<Vector2<Robot>, "fall_state_estimation.falling_angle_threshold_forward">,
    _falling_angle_threshold_forward_with_catching_steps: Parameter<
        Vector2<Robot>,
        "fall_state_estimation.falling_angle_threshold_forward_with_catching_steps",
    >,
    _falling_timeout: Parameter<Duration, "fall_state_estimation.falling_timeout">,
    _gravitational_acceleration_threshold:
        Parameter<f32, "fall_state_estimation.gravitational_acceleration_threshold">,
    _gravitational_force_sitting:
        Parameter<Vector3<Robot>, "fall_state_estimation.gravitational_force_sitting">,
    _gravity_acceleration: Parameter<f32, "physical_constants.gravity_acceleration">,
    _sitting_pose: Parameter<Joints<f32>, "fall_state_estimation.sitting_pose">,
    _catching_steps_enabled: Parameter<bool, "walking_engine.catching_steps.enabled">,

    _robot_orientation: RequiredInput<Option<Orientation3<Field>>, "robot_orientation?">,
    _sensor_data: Input<SensorData, "sensor_data">,
    _cycle_time: Input<CycleTime, "cycle_time">,
    _has_ground_contact: Input<bool, "has_ground_contact">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub fall_state: MainOutput<FallState>,
}

impl FallStateDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_fall_state: Default::default(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl PathsInterface>) -> Result<MainOutputs> {
        // let cycle_start = context.cycle_time.start_time;
        // let inertial_measurement_unit = context.sensor_data.inertial_measurement_unit;
        // let (roll, pitch, _) = context.robot_orientation.inner.euler_angles();

        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;

        let model_path = neural_network_folder.join("base_model.tflite");

        let model = FlatBufferModel::build_from_file(model_path)?;

        let resolver = BuiltinOpResolver::default();

        let builder = InterpreterBuilder::new(model, &resolver)?;
        let mut interpreter = builder.build()?;

        interpreter.allocate_tensors()?;

        let inputs = interpreter.inputs().to_vec();
        // assert_eq!(inputs.len(), 1);

        let _input_index = inputs[0];

        let outputs = interpreter.outputs().to_vec();
        // assert_eq!(outputs.len(), 1);

        let output_index = outputs[0];

        // let input_tensor = interpreter.tensor_info(input_index).unwrap();
        // // assert_eq!(input_tensor.dims, vec![1, 28, 28, 1]);

        // let output_tensor = interpreter.tensor_info(output_index).unwrap();
        // // assert_eq!(output_tensor.dims, vec![1, 10]);

        interpreter.print_state();

        interpreter.invoke()?;

        interpreter.print_state();

        let output: &[f32] = interpreter.tensor_data(output_index)?;

        dbg!(output);

        // let guess = output
        //     .iter()
        //     .enumerate()
        //     .max_by(|x, y| x.1.cmp(y.1))
        //     .unwrap()
        //     .0;

        // dbg!(guess);

        Ok(MainOutputs {
            fall_state: FallState::default().into(),
        })
    }
}
