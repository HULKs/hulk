use std::{
    fs::File,
    io::{stdin, BufReader},
    path::PathBuf,
    sync::Arc,
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
use types::{FieldDimensions, GameControllerState, PrimaryState};

#[derive(Parser)]
struct Arguments {
    #[arg(short, long, default_value = "[::]:1337")]
    listen_address: String,
    log_file: PathBuf,
}

fn main() -> Result<()> {
    let arguments = Arguments::parse();

    let (keep_running, writer, notifier) = start_communication_server(arguments.listen_address)?;

    let reader = BufReader::new(File::open(arguments.log_file)?);

    recording_player(reader, writer, notifier)?;
    keep_running.cancel();

    Ok(())
}

fn recording_player(
    mut reader: BufReader<File>,
    writer: Writer<Database>,
    notifier: Arc<Notify>,
) -> Result<()> {
    while let Ok(data) = deserialize_from::<_, RecordedCycleContext>(&mut reader) {
        {
            let mut database = writer.next();
            println!("{data:?}");

            database.main_outputs.game_controller_state = data.game_controller_state;
            database.main_outputs.has_ground_contact = data.has_ground_contact;
            database.main_outputs.primary_state = data.primary_state;
            database.main_outputs.robot_to_field = data.robot_to_field;
            database.main_outputs.robot_to_field = data.robot_to_field;
        }
        notifier.notify_waiters();

        stdin().read_line(&mut String::new()).unwrap();
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
struct Parameters {
    field_dimensions: FieldDimensions,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
struct Database {
    main_outputs: MainOutputs,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
struct MainOutputs {
    pub game_controller_state: Option<GameControllerState>,
    pub has_ground_contact: bool,
    pub primary_state: PrimaryState,
    pub robot_to_field: Option<Isometry2<f32>>,
}

fn start_communication_server(
    listen_address: String,
) -> Result<(CancellationToken, Writer<Database>, Arc<Notify>)> {
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

    let (outputs_writer, outputs_reader) =
        multiple_buffer_with_slots([Database::default(), Default::default(), Default::default()]);
    let outputs_changed = Arc::new(Notify::new());
    let (subscribed_outputs_writer, _subscribed_outputs_reader) =
        multiple_buffer_with_slots([Default::default(), Default::default(), Default::default()]);

    communication_server.register_cycler_instance(
        "Control",
        outputs_changed.clone(),
        outputs_reader,
        subscribed_outputs_writer,
    );

    Ok((keep_running, outputs_writer, outputs_changed))
}
