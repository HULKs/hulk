use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    io::BufReader,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime},
};

use bincode::deserialize_from;
use clap::Parser;
use color_eyre::Result;
use communication::server::Runtime;
use control::{
    localization::{
        get_fitted_field_mark_correspondence,
        goal_support_structure_line_marks_from_field_dimensions,
    },
    localization_recorder::RecordedCycleContext,
};
use framework::{multiple_buffer_with_slots, Reader, Writer};
use nalgebra::Isometry2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use tokio::{select, sync::Notify, time::interval};
use tokio_util::sync::CancellationToken;
use types::{
    field_marks_from_field_dimensions, FieldDimensions, FieldMark, GameControllerState, Line,
    Line2, LineData, PrimaryState,
};

#[derive(Parser)]
struct Arguments {
    #[arg(short, long, default_value = "[::]:1337")]
    listen_address: String,
    log_file: PathBuf,
}

fn main() -> Result<()> {
    let arguments = Arguments::parse();

    let (
        keep_running,
        simulator_writer,
        control_writer,
        vision_top_writer,
        vision_bottom_writer,
        database_changed,
        parameters_reader,
        parameters_changed,
    ) = start_communication_server(arguments.listen_address)?;

    let reader = BufReader::new(File::open(arguments.log_file)?);

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(recording_player(
        reader,
        simulator_writer,
        control_writer,
        vision_top_writer,
        vision_bottom_writer,
        database_changed,
        parameters_reader,
        parameters_changed,
    ))?;
    keep_running.cancel();

    Ok(())
}

fn merge_line_data(line_data: &BTreeMap<SystemTime, Vec<Option<LineData>>>) -> LineData {
    let lines_in_robot = line_data
        .values()
        .flatten()
        .flatten()
        .flat_map(|line_data| line_data.lines_in_robot.clone())
        .collect();
    LineData {
        lines_in_robot,
        used_vertical_filtered_segments: HashSet::new(),
    }
}

fn line_correspondences(
    lines: &LineData,
    robot_to_field: Isometry2<f32>,
    field_marks: &[FieldMark],
) -> Vec<Line2> {
    let lines: Vec<Line2> = lines
        .lines_in_robot
        .iter()
        .map(|line| Line(robot_to_field * line.0, robot_to_field * line.1))
        .collect();
    let (correspondences, fit_error, fit_errors) =
        get_fitted_field_mark_correspondence(&lines, &field_marks, 1e-2, 0.01, 1.5, 20, 10, true);
    correspondences
        .iter()
        .flat_map(|field_mark_correspondence| {
            let correspondence_points_0 = field_mark_correspondence.correspondence_points.0;
            let correspondence_points_1 = field_mark_correspondence.correspondence_points.1;
            [
                Line(
                    correspondence_points_0.measured,
                    correspondence_points_0.reference,
                ),
                Line(
                    correspondence_points_1.measured,
                    correspondence_points_1.reference,
                ),
            ]
        })
        .collect()
}

fn generate_field_marks(field_dimensions: FieldDimensions) -> Vec<FieldMark> {
    field_marks_from_field_dimensions(&field_dimensions)
        .into_iter()
        .chain(goal_support_structure_line_marks_from_field_dimensions(
            &field_dimensions,
        ))
        .collect()
}

#[allow(clippy::too_many_arguments)]
async fn recording_player(
    mut reader: BufReader<File>,
    simulator_writer: Writer<SimulatorDatabase>,
    control_writer: Writer<ControlDatabase>,
    vision_top_writer: Writer<VisionDatabase>,
    vision_bottom_writer: Writer<VisionDatabase>,
    database_changed: Arc<Notify>,
    parameters_reader: Reader<Parameters>,
    parameters_changed: Arc<Notify>,
) -> Result<()> {
    let mut frames = Vec::new();
    while let Ok(data) = deserialize_from::<_, RecordedCycleContext>(&mut reader) {
        frames.push(data);
    }
    {
        simulator_writer.next().main_outputs.frame_count = frames.len();
    }
    let mut interval = interval(Duration::from_secs(1));
    let field_marks = generate_field_marks(parameters_reader.next().field_dimensions.clone());

    loop {
        select! {
            _ = parameters_changed.notified() => { }
            _ = interval.tick() => { }
        }

        let parameters = parameters_reader.next();

        {
            let data = &frames[parameters.selected_frame];
            let lines_bottom = merge_line_data(&data.line_data_top_persistent);
            let lines_top = merge_line_data(&data.line_data_bottom_persistent);
            let correspondence_lines_top = line_correspondences(
                &lines_top,
                data.robot_to_field.unwrap_or_default(),
                &field_marks,
            );
            let correspondence_lines_bottom = line_correspondences(
                &lines_bottom,
                data.robot_to_field.unwrap_or_default(),
                &field_marks,
            );

            {
                let mut database = control_writer.next();

                database.main_outputs.game_controller_state = data.game_controller_state;
                database.main_outputs.has_ground_contact = data.has_ground_contact;
                database.main_outputs.primary_state = data.primary_state;
                database.main_outputs.robot_to_field = data.robot_to_field;
            }
            {
                let mut database = vision_top_writer.next();
                database.main_outputs.line_data = Some(lines_top);
                database
                    .additional_outputs
                    .localization
                    .correspondence_lines = correspondence_lines_top
            }
            {
                let mut database = vision_bottom_writer.next();
                database.main_outputs.line_data = Some(lines_bottom);
                database
                    .additional_outputs
                    .localization
                    .correspondence_lines = correspondence_lines_bottom
            }
        }
        database_changed.notify_waiters();
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
struct Parameters {
    field_dimensions: FieldDimensions,
    selected_frame: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
struct SimulatorMainOutputs {
    frame_count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
struct SimulatorDatabase {
    main_outputs: SimulatorMainOutputs,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
struct ControlDatabase {
    main_outputs: ControlMainOutputs,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
struct ControlMainOutputs {
    pub game_controller_state: Option<GameControllerState>,
    pub has_ground_contact: bool,
    pub primary_state: PrimaryState,
    pub robot_to_field: Option<Isometry2<f32>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
struct VisionDatabase {
    main_outputs: VisionMainOutputs,
    additional_outputs: VisionAdditionalOutputs,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
struct VisionMainOutputs {
    line_data: Option<LineData>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
struct VisionAdditionalOutputs {
    localization: LocalizationAdditionalOutputs,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
struct LocalizationAdditionalOutputs {
    correspondence_lines: Vec<Line2>,
}

#[allow(clippy::type_complexity)]
fn start_communication_server(
    listen_address: String,
) -> Result<(
    CancellationToken,
    Writer<SimulatorDatabase>,
    Writer<ControlDatabase>,
    Writer<VisionDatabase>,
    Writer<VisionDatabase>,
    Arc<Notify>,
    Reader<Parameters>,
    Arc<Notify>,
)> {
    let parameter_slots = 3;

    let keep_running = CancellationToken::new();

    let communication_server = Runtime::<Parameters>::start(
        Some(listen_address),
        ".",
        "".to_string(),
        "".to_string(),
        parameter_slots,
        keep_running.clone(),
    )?;

    let (simulator_writer, simulator_reader) =
        multiple_buffer_with_slots([Default::default(), Default::default(), Default::default()]);
    let (control_writer, control_reader) =
        multiple_buffer_with_slots([Default::default(), Default::default(), Default::default()]);
    let (vision_top_writer, vision_top_reader) =
        multiple_buffer_with_slots([Default::default(), Default::default(), Default::default()]);
    let (vision_bottom_writer, vision_bottom_reader) =
        multiple_buffer_with_slots([Default::default(), Default::default(), Default::default()]);
    let database_changed = Arc::new(Notify::new());

    let (subscribed_simulator_writer, _subscribed_simulator_reader) =
        multiple_buffer_with_slots([Default::default(), Default::default(), Default::default()]);
    communication_server.register_cycler_instance(
        "BehaviorSimulator",
        database_changed.clone(),
        simulator_reader,
        subscribed_simulator_writer,
    );

    let (subscribed_control_writer, _subscribed_control_reader) =
        multiple_buffer_with_slots([Default::default(), Default::default(), Default::default()]);
    communication_server.register_cycler_instance(
        "Control",
        database_changed.clone(),
        control_reader,
        subscribed_control_writer,
    );

    let (subscribed_vision_top_writer, _subscribed_vision_top_reader) =
        multiple_buffer_with_slots([Default::default(), Default::default(), Default::default()]);
    communication_server.register_cycler_instance(
        "VisionTop",
        database_changed.clone(),
        vision_top_reader,
        subscribed_vision_top_writer,
    );
    let (subscribed_vision_bottom_writer, _subscribed_vision_bottom_reader) =
        multiple_buffer_with_slots([Default::default(), Default::default(), Default::default()]);
    communication_server.register_cycler_instance(
        "VisionBottom",
        database_changed.clone(),
        vision_bottom_reader,
        subscribed_vision_bottom_writer,
    );

    Ok((
        keep_running,
        simulator_writer,
        control_writer,
        vision_top_writer,
        vision_bottom_writer,
        database_changed,
        communication_server.get_parameters_reader(),
        communication_server.get_parameters_changed(),
    ))
}
