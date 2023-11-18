use std::time::Duration;

use hulk_behavior_simulator::{
    simulator::Scenario,
    state::{Ball, State},
};
use spl_network_messages::{GameState, PlayerNumber};

#[derive(Default)]
struct GoldenGoal {
    goals: u32,
}

impl Scenario for GoldenGoal {
    fn init(&mut self, state: &mut State) -> color_eyre::eyre::Result<()> {
        state.spawn_robot(PlayerNumber::One)?;
        state.spawn_robot(PlayerNumber::Two)?;
        state.spawn_robot(PlayerNumber::Three)?;
        state.spawn_robot(PlayerNumber::Four)?;
        state.spawn_robot(PlayerNumber::Five)?;
        state.spawn_robot(PlayerNumber::Six)?;
        state.spawn_robot(PlayerNumber::Seven)?;

        Ok(())
    }

    fn cycle(&mut self, state: &mut State) -> color_eyre::eyre::Result<()> {
        let cycle = state.cycle_count;
        if cycle == 100 {
            for robot in state.robots.values_mut() {
                robot.is_penalized = false;
                state.ball.as_ref().inspect(|ball| {
                    println!("{:?}", ball.position);
                });
            }
            state.game_controller_state.game_state = GameState::Ready;
        }
        if cycle == 1600 {
            state.game_controller_state.game_state = GameState::Set;
        }
        if cycle == 1700 {
            state.ball = Some(Ball::default());
        }

        if cycle % 2000 == 0 {
            state.game_controller_state.game_state = GameState::Playing;
        }

        if state
            .ball
            .as_ref()
            .is_some_and(|ball| ball.position.x().abs() > 4.5 && ball.position.y() < 0.75)
        {
            self.goals += 1;
            state.ball = None;
            state.game_controller_state.game_state = GameState::Set;
        }
        if self.goals > 0 {
            state.finished = true;
        }

        println!("{}", state.cycle_count);
        state.cycle(Duration::from_millis(12))?;

        Ok(())
    }
}

fn main() -> color_eyre::Result<()> {
    let mut scenario = GoldenGoal::default();
    return scenario.run();

    // let mut simulator = Recorder::default();
    // let mut state = State::default();
    //
    // for player_number in [
    //     PlayerNumber::One,
    //     PlayerNumber::Two,
    //     PlayerNumber::Three,
    //     PlayerNumber::Four,
    //     PlayerNumber::Five,
    //     PlayerNumber::Six,
    //     PlayerNumber::Seven,
    // ] {
    //     state.spawn_robot(player_number)?;
    // }
    //
    // let mut goals = 0;
    //
    // let start = Instant::now();
    // for cycle in 0..10000 {
    //     if cycle == 100 {
    //         for robot in state.robots.values_mut() {
    //             robot.is_penalized = false;
    //         }
    //         state.game_state = FilteredGameState::Ready {
    //             kicking_team: Team::Hulks,
    //         }
    //     }
    //     if cycle == 1600 {
    //         state.game_state = FilteredGameState::Set;
    //     }
    //     if cycle == 1700 {
    //         state.ball = Some(Ball::default());
    //     }
    //
    //     if cycle % 2000 == 0 {
    //         state.game_state = FilteredGameState::Playing { ball_is_free: true };
    //     }
    //
    //     if state
    //         .ball
    //         .as_ref()
    //         .is_some_and(|ball| ball.position.x.abs() > 4.5 && ball.position.y < 0.75)
    //     {
    //         goals += 1;
    //         state.ball = None;
    //         state.game_state = FilteredGameState::Set;
    //     }
    //     if goals > 0 {
    //         break;
    //     }
    //
    //     state.cycle(Duration::from_millis(12))?;
    //     simulator.record(&mut state);
    // }
    // let duration = Instant::now() - start;
    // println!("Took {:.2} seconds", duration.as_secs_f32());
    // println!("Frames: {}", simulator.frames.len());
    //
    // server::run(
    //     simulator.frames,
    //     Some("[::]:1337"),
    //     CancellationToken::new(),
    // )?;

    Ok(())
}
