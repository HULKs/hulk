use std::collections::VecDeque;

use color_eyre::Result;
use hardware::PathsInterface;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::{deserialize_not_implemented, MainOutput};
use tflite::{ops::builtin::BuiltinOpResolver, FlatBufferModel, InterpreterBuilder};
use types::{
    fall_state::{FallState, FallStateTinyML},
    sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct FallStateDetection {
    last_fall_state: FallState,
    #[serde(skip, default = "deserialize_not_implemented")]
    model: FlatBufferModel,
    datas: Vec<VecDeque<f32>>,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    sensor_data: Input<SensorData, "sensor_data">,
    has_ground_contact: Input<bool, "has_ground_contact">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub fall_state_tinyml: MainOutput<FallStateTinyML>,
    pub soft_fall_state_tinyml: MainOutput<f32>,
}

impl FallStateDetection {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;

        let model_path = neural_network_folder.join("fall_detection_shift_23.tflite");

        let model = FlatBufferModel::build_from_file(model_path)?;

        Ok(Self {
            last_fall_state: Default::default(),
            model,
            datas: vec![VecDeque::new(); 6],
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        // TODO: hadle primary state unstiff

        let inertial_measurement_unit = context.sensor_data.inertial_measurement_unit;
        let linear_accelerations = inertial_measurement_unit.linear_acceleration.inner;
        let roll_pitch = inertial_measurement_unit.roll_pitch.inner;
        let has_ground_contact = if *context.has_ground_contact {
            1.0
        } else {
            0.0
        };

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

        self.datas[0].push_back(linear_accelerations.x);
        self.datas[1].push_back(linear_accelerations.y);
        self.datas[2].push_back(linear_accelerations.z);
        self.datas[3].push_back(roll_pitch.x);
        self.datas[4].push_back(roll_pitch.y);
        self.datas[5].push_back(has_ground_contact);

        let max_size = input_tensor.dims[1];
        for i in 0..input_tensor.dims[2] {
            while self.datas[i].len() > max_size {
                self.datas[i].pop_front();
            }
        }

        dbg!(input_tensor.dims);

        if self.datas[0].len() == max_size {
            interpreter.tensor_data_mut(input_index).unwrap()[0..max_size]
                .copy_from_slice(self.datas[0].make_contiguous());
            interpreter.tensor_data_mut(input_index).unwrap()[max_size..max_size * 2]
                .copy_from_slice(self.datas[1].make_contiguous());
            interpreter.tensor_data_mut(input_index).unwrap()[max_size * 2..max_size * 3]
                .copy_from_slice(self.datas[2].make_contiguous());
            interpreter.tensor_data_mut(input_index).unwrap()[max_size * 3..max_size * 4]
                .copy_from_slice(self.datas[3].make_contiguous());
            interpreter.tensor_data_mut(input_index).unwrap()[max_size * 4..max_size * 5]
                .copy_from_slice(self.datas[4].make_contiguous());
            interpreter.tensor_data_mut(input_index).unwrap()[max_size * 5..max_size * 6]
                .copy_from_slice(self.datas[5].make_contiguous());
        }

        interpreter.invoke()?;

        let output: &[f32] = interpreter.tensor_data(output_index)?;
        assert!(output.len() == 1);

        let guess = *output.first().unwrap();

        let fall_state = if guess <= 5.0 {
            FallStateTinyML::Stable
        } else {
            FallStateTinyML::SoonToBeUnstable
        };

        Ok(MainOutputs {
            fall_state_tinyml: fall_state.into(),
            soft_fall_state_tinyml: guess.into(),
        })
    }
}
