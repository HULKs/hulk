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
use framework::{multiple_buffer_with_slots, Writer};
use nalgebra::Isometry2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use tokio::sync::Notify;
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

    let (keep_running, control_writer, vision_top_writer, vision_bottom_writer, notifier) =
        start_communication_server(arguments.listen_address)?;

    let reader = BufReader::new(File::open(arguments.log_file)?);

    recording_player(
        reader,
        control_writer,
        vision_top_writer,
        vision_bottom_writer,
        notifier,
    )?;
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

fn recording_player(
    mut reader: BufReader<File>,
    control_writer: Writer<ControlDatabase>,
    vision_top_writer: Writer<VisionDatabase>,
    vision_bottom_writer: Writer<VisionDatabase>,
    notifier: Arc<Notify>,
) -> Result<()> {
    while let Ok(data) = deserialize_from::<_, RecordedCycleContext>(&mut reader) {
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
            database.main_outputs.line_data = Some(merge_line_data(&data.line_data_top_persistent));
        }
        {
            let mut database = vision_bottom_writer.next();
            database.main_outputs.line_data =
                Some(merge_line_data(&data.line_data_bottom_persistent));
        }

        notifier.notify_waiters();

        // stdin().read_line(&mut String::new()).unwrap();
        thread::sleep(Duration::from_millis(12));
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
struct Parameters {
    field_dimensions: FieldDimensions,
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
    Writer<ControlDatabase>,
    Writer<VisionDatabase>,
    Writer<VisionDatabase>,
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

    let (control_writer, control_reader) = multiple_buffer_with_slots([
        ControlDatabase::default(),
        Default::default(),
        Default::default(),
    ]);
    let (vision_top_writer, vision_top_reader) = multiple_buffer_with_slots([
        VisionDatabase::default(),
        Default::default(),
        Default::default(),
    ]);
    let (vision_bottom_writer, vision_bottom_reader) = multiple_buffer_with_slots([
        VisionDatabase::default(),
        Default::default(),
        Default::default(),
    ]);
    let database_changed = Arc::new(Notify::new());
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
    let (subscribed_vision_bottom_writer, _subscribed_control_reader) =
        multiple_buffer_with_slots([Default::default(), Default::default(), Default::default()]);
    communication_server.register_cycler_instance(
        "VisionBottom",
        database_changed.clone(),
        vision_bottom_reader,
        subscribed_vision_bottom_writer,
    );

    Ok((
        keep_running,
        control_writer,
        vision_top_writer,
        vision_bottom_writer,
        database_changed,
    ))
}
