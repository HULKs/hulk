use std::time::{SystemTime, UNIX_EPOCH};

use bevy::{
    app::{App, PostUpdate},
    ecs::system::{Query, Res, ResMut, Resource},
    time::Time,
};
use color_eyre::Result;
use tokio::{
    runtime::Runtime,
    sync::mpsc::{self, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use types::{ball_position::SimulatorBallState, players::Players};

use crate::{ball::BallResource, cyclers::control::Database, robot::Robot, server};

pub struct Frame {
    pub timestamp: SystemTime,
    pub ball: Option<SimulatorBallState>,
    pub robots: Players<Option<Database>>,
}

#[derive(Resource)]
pub struct Recording {
    frame_sender: UnboundedSender<Frame>,
    join_handle: JoinHandle<Result<()>>,
    runtime: Runtime,
}

impl Default for Recording {
    fn default() -> Self {
        let (frame_sender, frame_receiver) = mpsc::unbounded_channel();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let join_handle = runtime.spawn(server::run(
            frame_receiver,
            "[::]:1337",
            CancellationToken::new(),
        ));
        Self {
            frame_sender,
            join_handle,
            runtime,
        }
    }
}

pub fn frame_recorder(
    robots: Query<&Robot>,
    ball: Res<BallResource>,
    recording: ResMut<Recording>,
    time: Res<Time>,
) {
    let mut players = Players::<Option<Database>>::default();
    for robot in &robots {
        players[robot.parameters.player_number] = Some(robot.database.clone())
    }
    recording
        .frame_sender
        .send(Frame {
            timestamp: UNIX_EPOCH + time.elapsed(),
            robots: players,
            ball: ball.state,
        })
        .expect("failed to send frame to server");
}

impl Recording {
    pub fn join(self) -> Result<()> {
        let Self {
            frame_sender,
            join_handle,
            runtime,
        } = self;
        drop(frame_sender);
        runtime.block_on(join_handle)?
    }
}

pub fn recording_plugin(app: &mut App) {
    app.insert_resource(Recording::default())
        .add_systems(PostUpdate, frame_recorder);
}
