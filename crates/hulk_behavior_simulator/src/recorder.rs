use bevy::{
    app::{App, PostUpdate},
    ecs::system::{Query, Res, ResMut, Resource},
};
use color_eyre::Result;
use tokio_util::sync::CancellationToken;

use types::{ball_position::SimulatorBallState, players::Players};

use crate::{ball::BallResource, cyclers::control::Database, robot::Robot, server};

pub struct Frame {
    pub ball: Option<SimulatorBallState>,
    pub robots: Players<Option<Database>>,
}

#[derive(Resource, Default)]
pub struct Recording {
    pub frames: Vec<Frame>,
}

pub fn frame_recorder(
    robots: Query<&Robot>,
    ball: Res<BallResource>,
    mut recording: ResMut<Recording>,
) {
    let mut players = Players::<Option<Database>>::default();
    for robot in &robots {
        players[robot.parameters.player_number] = Some(robot.database.clone())
    }
    recording.frames.push(Frame {
        robots: players,
        ball: ball.state,
    });
}

impl Recording {
    pub fn serve(&mut self) -> Result<()> {
        server::run(
            std::mem::take(&mut self.frames),
            "[::]:1337",
            CancellationToken::new(),
        )
    }
}

pub fn recording_plugin(app: &mut App) {
    app.insert_resource(Recording::default())
        .add_systems(PostUpdate, frame_recorder);
}
