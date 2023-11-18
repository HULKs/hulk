use std::time::{Duration, SystemTime};

use crate::{cyclers::control::Database, server};
use color_eyre::Result;
use tokio_util::sync::CancellationToken;
use types::{ball_position::SimulatorBallState, players::Players};

use crate::state::State;

pub struct Frame {
    pub ball: Option<SimulatorBallState>,
    pub robots: Players<Option<Database>>,
}

pub trait Scenario {
    fn init(&mut self, state: &mut State) -> Result<()>;

    fn cycle(&mut self, state: &mut State) -> Result<()> {
        state.cycle(Duration::from_millis(12))
    }

    fn run(&mut self) -> Result<()> {
        let mut state = State::default();
        self.init(&mut state)?;
        let mut recorder = Recorder::default();
        let start = SystemTime::now();
        while self.should_continue(&mut state) {
            self.cycle(&mut state)?;
            recorder.record(&mut state);
        }
        let duration = start.elapsed().expect("time ran backwards");
        println!("Took {:.2}s", duration.as_secs_f32());
        let cycles = state.cycle_count;
        println!(
            "{} cycles, {:.2} cycles/s",
            cycles,
            cycles as f32 / duration.as_secs_f32()
        );

        server::run(recorder.frames, "[::]:1337", CancellationToken::new())?;

        Ok(())
    }

    fn should_continue(&mut self, state: &mut State) -> bool {
        !state.finished && state.cycle_count < 10_000
    }
}

#[derive(Default)]
pub struct Recorder {
    pub frames: Vec<Frame>,
}

impl Recorder {
    pub fn record(&mut self, state: &mut State) {
        let mut robots = Players::<Option<Database>>::default();
        for (player_number, robot) in &state.robots {
            robots[*player_number] = Some(robot.database.clone())
        }
        self.frames.push(Frame {
            robots,
            ball: state.ball,
        });
    }
}
