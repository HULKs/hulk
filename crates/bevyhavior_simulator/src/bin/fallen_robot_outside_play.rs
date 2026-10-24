use bevy::prelude::*;
use bevyhavior_simulator::behavior_tree_simulator::{
    BehaviorTreeSimulatorSet, SimulatorFieldDimensions, SimulatorPosture, SimulatorPrimaryState,
    SimulatorRobotBundle, SimulatorRobotFrames, default_behavior_parameters,
    sync_primary_states_from_game_state,
};
use hsl_network_messages::{PlayerNumber, Team};
use linear_algebra::{Isometry2, vector};
use scenario::scenario;
use types::{fall_detection::Posture, motion_command::MotionCommand, primary_state::PrimaryState};

#[derive(Clone, Copy, Debug)]
enum ExpectedMotion {
    Damping,
    Prepare,
    Stand,
    StandUp,
}

impl ExpectedMotion {
    fn matches(self, motion_command: &MotionCommand) -> bool {
        matches!(
            (self, motion_command),
            (Self::Damping, MotionCommand::Damping)
                | (Self::Prepare, MotionCommand::Prepare)
                | (Self::Stand, MotionCommand::Stand { .. })
                | (Self::StandUp, MotionCommand::StandUp)
        )
    }
}

type Case = (PrimaryState, Posture, ExpectedMotion);

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for primary_state in [
        PrimaryState::Initial,
        PrimaryState::Stop,
        PrimaryState::Penalized,
        PrimaryState::Finished,
    ] {
        cases.extend([
            (primary_state, Posture::Upright, ExpectedMotion::Stand),
            (primary_state, Posture::Falling, ExpectedMotion::Damping),
            (
                primary_state,
                Posture::Fallen {
                    is_ready_for_standup: false,
                },
                ExpectedMotion::Damping,
            ),
            (
                primary_state,
                Posture::Fallen {
                    is_ready_for_standup: true,
                },
                ExpectedMotion::Damping,
            ),
            (primary_state, Posture::StandingUp, ExpectedMotion::StandUp),
        ]);
    }
    cases.extend([
        (
            PrimaryState::Playing,
            Posture::Fallen {
                is_ready_for_standup: false,
            },
            ExpectedMotion::Prepare,
        ),
        (
            PrimaryState::Playing,
            Posture::Fallen {
                is_ready_for_standup: true,
            },
            ExpectedMotion::StandUp,
        ),
        (
            PrimaryState::Damping,
            Posture::StandingUp,
            ExpectedMotion::Damping,
        ),
        (
            PrimaryState::Prepare,
            Posture::Fallen {
                is_ready_for_standup: true,
            },
            ExpectedMotion::Prepare,
        ),
    ]);
    cases
}

#[derive(Resource, Default)]
struct CurrentCase(usize);

#[scenario]
fn fallen_robot_outside_play(app: &mut App) {
    app.init_resource::<CurrentCase>()
        .add_systems(Startup, startup)
        .add_systems(
            Update,
            apply_case
                .in_set(BehaviorTreeSimulatorSet::BeforeWorldState)
                .after(sync_primary_states_from_game_state),
        )
        .add_systems(Update, update.in_set(BehaviorTreeSimulatorSet::Scenario));
}

fn startup(mut commands: Commands, field_dimensions: Res<SimulatorFieldDimensions>) {
    let parameters =
        default_behavior_parameters().expect("failed to load default behavior parameters");

    commands.spawn(
        SimulatorRobotBundle::new(
            Team::Hulks,
            PlayerNumber::Three,
            Isometry2::from_parts(vector![-1.0, 0.0], 0.0),
            field_dimensions.0,
            parameters,
        )
        .expect("failed to create robot bundle"),
    );
}

fn apply_case(
    current_case: Res<CurrentCase>,
    mut robots: Query<(&mut SimulatorPrimaryState, &mut SimulatorPosture)>,
) {
    let Some((primary_state, posture, _)) = cases().get(current_case.0).copied() else {
        return;
    };
    for (mut robot_primary_state, mut robot_posture) in &mut robots {
        robot_primary_state.primary_state = primary_state;
        robot_posture.posture = Some(posture);
    }
}

fn update(
    robot_frames: Res<SimulatorRobotFrames>,
    mut current_case: ResMut<CurrentCase>,
    mut exit: MessageWriter<AppExit>,
) {
    let cases = cases();
    let (primary_state, posture, expected_motion) = cases[current_case.0];

    if robot_frames.0.is_empty() {
        println!("failed: no robot frames recorded");
        exit.write(AppExit::from_code(1));
        return;
    }
    for robot_frame in robot_frames.0.values() {
        let motion_command = &robot_frame.motion_command;
        if !expected_motion.matches(motion_command) {
            println!(
                "failed: {primary_state:?} with {posture:?} commanded {motion_command:?}, expected {expected_motion:?}"
            );
            exit.write(AppExit::from_code(1));
            return;
        }
    }

    current_case.0 += 1;
    if current_case.0 == cases.len() {
        println!("ok: {} cases", cases.len());
        exit.write(AppExit::Success);
    }
}
