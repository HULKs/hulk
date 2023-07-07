use std::{
    collections::{BTreeMap, HashSet},
    fs::File,
    io::{stdin, BufReader},
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, SystemTime},
};

use bincode::deserialize_from;
use clap::Parser;
use color_eyre::Result;
use communication::server::Runtime;
use control::localization_recorder::RecordedCycleContext;
use framework::{multiple_buffer_with_slots, Reader, Writer};
use nalgebra::Isometry2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use tokio::{select, sync::Notify, time::interval};
use tokio_util::sync::CancellationToken;
use types::{FieldDimensions, GameControllerState, LineData, PrimaryState};

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

    loop {
        select! {
            _ = parameters_changed.notified() => { }
            _ = interval.tick() => { }
        }

        let parameters = parameters_reader.next();

        {
            let data = &frames[parameters.selected_frame];
            {
                let mut database = control_writer.next();

                database.main_outputs.game_controller_state = data.game_controller_state;
                database.main_outputs.has_ground_contact = data.has_ground_contact;
                database.main_outputs.primary_state = data.primary_state;
                database.main_outputs.robot_to_field = data.robot_to_field;
                database.main_outputs.robot_to_field = data.robot_to_field;
            }
            {
                let mut database = vision_top_writer.next();
                database.main_outputs.line_data =
                    Some(merge_line_data(&data.line_data_top_persistent));
            }
            {
                let mut database = vision_bottom_writer.next();
                database.main_outputs.line_data =
                    Some(merge_line_data(&data.line_data_bottom_persistent));
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
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
struct VisionMainOutputs {
    line_data: Option<LineData>,
}

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
