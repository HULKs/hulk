use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    io::{BufWriter, Write},
    time::{SystemTime, UNIX_EPOCH},
};

use bincode::serialize;
use color_eyre::{eyre::Context, Result};
use context_attribute::context;
use framework::{HistoricInput, PerceptionInput};
use nalgebra::Isometry2;
use serde::{Deserialize, Serialize};
use types::{
    game_controller_state::GameControllerState, line_data::LineData, primary_state::PrimaryState,
};

pub struct LocalizationRecorder {
    recording: Option<BufWriter<File>>,
}

#[context]
pub struct CreationContext {
    enable: Parameter<bool, "localization_recorder.enable">,
}

#[context]
pub struct CycleContext {
    enable: Parameter<bool, "localization_recorder.enable">,
    only_record_during_active_localization:
        Parameter<bool, "localization_recorder.only_record_during_active_localization">,

    current_odometry_to_last_odometry:
        HistoricInput<Option<Isometry2<f32>>, "current_odometry_to_last_odometry?">,

    game_controller_state: Input<Option<GameControllerState>, "game_controller_state?">,
    has_ground_contact: Input<bool, "has_ground_contact">,
    primary_state: Input<PrimaryState, "primary_state">,
    robot_to_field: Input<Option<Isometry2<f32>>, "robot_to_field?">,

    line_data_bottom: PerceptionInput<Option<LineData>, "VisionBottom", "line_data?">,
    line_data_top: PerceptionInput<Option<LineData>, "VisionTop", "line_data?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl LocalizationRecorder {
    pub fn new(context: CreationContext) -> Result<Self> {
        if *context.enable {
            let seconds = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            Ok(Self {
                recording: Some(BufWriter::new(
                    File::create(format!("logs/localization.{seconds}.bincode"))
                        .wrap_err("failed")?,
                )),
            })
        } else {
            Ok(Self { recording: None })
        }
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        if !*context.enable {
            return Ok(MainOutputs::default());
        }

        if *context.only_record_during_active_localization
            && !matches!(
                context.primary_state,
                PrimaryState::Ready
                    | PrimaryState::Set
                    | PrimaryState::Playing
                    | PrimaryState::Calibration
            )
        {
            return Ok(MainOutputs::default());
        }

        let timestamps = context
            .line_data_top
            .persistent
            .keys()
            .chain(context.line_data_top.temporary.keys())
            .chain(context.line_data_bottom.persistent.keys())
            .chain(context.line_data_bottom.temporary.keys())
            .collect::<HashSet<_>>();
        let current_odometry_to_last_odometry = timestamps
            .into_iter()
            .map(|timestamp| {
                (
                    *timestamp,
                    context
                        .current_odometry_to_last_odometry
                        .get(timestamp)
                        .cloned(),
                )
            })
            .collect();
        let recorded_context = RecordedCycleContext {
            current_odometry_to_last_odometry,
            game_controller_state: context.game_controller_state.cloned(),
            has_ground_contact: *context.has_ground_contact,
            primary_state: *context.primary_state,
            robot_to_field: context.robot_to_field.cloned(),
            line_data_bottom_persistent: context
                .line_data_bottom
                .persistent
                .iter()
                .map(|(key, value)| (*key, value.iter().map(|value| value.cloned()).collect()))
                .collect(),
            line_data_bottom_temporary: context
                .line_data_bottom
                .temporary
                .iter()
                .map(|(key, value)| (*key, value.iter().map(|value| value.cloned()).collect()))
                .collect(),
            line_data_top_persistent: context
                .line_data_top
                .persistent
                .iter()
                .map(|(key, value)| (*key, value.iter().map(|value| value.cloned()).collect()))
                .collect(),
            line_data_top_temporary: context
                .line_data_top
                .temporary
                .iter()
                .map(|(key, value)| (*key, value.iter().map(|value| value.cloned()).collect()))
                .collect(),
        };
        let buffer =
            serialize(&recorded_context).wrap_err("failed to serialize recorded context")?;
        self.recording
            .as_mut()
            .unwrap()
            .write(&buffer)
            .wrap_err("failed to write recorded context")?;
        Ok(MainOutputs::default())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RecordedCycleContext {
    pub current_odometry_to_last_odometry: BTreeMap<SystemTime, Option<Isometry2<f32>>>,

    pub game_controller_state: Option<GameControllerState>,
    pub has_ground_contact: bool,
    pub primary_state: PrimaryState,
    pub robot_to_field: Option<Isometry2<f32>>,

    pub line_data_bottom_persistent: BTreeMap<SystemTime, Vec<Option<LineData>>>,
    pub line_data_bottom_temporary: BTreeMap<SystemTime, Vec<Option<LineData>>>,
    pub line_data_top_persistent: BTreeMap<SystemTime, Vec<Option<LineData>>>,
    pub line_data_top_temporary: BTreeMap<SystemTime, Vec<Option<LineData>>>,
}
