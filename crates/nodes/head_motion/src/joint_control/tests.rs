use super::*;
use crate::parameters::Parameters;
use json5::from_str;
use serde::Deserialize;

fn configuration() -> (JointControlParameters, JointLimits) {
    #[derive(Deserialize)]
    struct Global {
        joint_limits: JointLimits,
    }
    let head: Parameters = from_str(include_str!(
        "../../../../../etc/parameters/base/head_motion.json5"
    ))
    .unwrap();
    let global: Global = from_str(include_str!(
        "../../../../../etc/parameters/base/global.json5"
    ))
    .unwrap();
    (head.joint_control, global.joint_limits)
}

fn observed(position: f32, velocity: f32) -> HeadObservation {
    HeadObservation {
        positions: HeadJoints::fill(position),
        velocities: HeadJoints::fill(velocity),
    }
}

#[test]
fn reversals_and_irregular_requests_preserve_derivative_bounds() {
    let (parameters, joints) = configuration();
    let mut controller = JointController::default();
    let mut observation = observed(0.0, 0.0);
    let mut now = Time::zero();
    let mut previous = controller
        .update(
            JointTarget::Position(HeadJoints::fill(0.7)),
            &observation,
            &parameters,
            &joints,
            now,
        )
        .unwrap()
        .reference;
    for step in 0..600 {
        let dt = Duration::from_millis([0, 2, 13, 20, 7, 31][step % 6]);
        now = now + dt;
        let target = match step {
            0..10 => 0.7,
            10..25 => -0.3,
            25..120 => 0.45 + 0.2 * (step as f32 * 0.07).sin(),
            _ => 0.0,
        };
        let output = controller
            .update(
                JointTarget::Position(HeadJoints::fill(target)),
                &observation,
                &parameters,
                &joints,
                now,
            )
            .unwrap();
        assert!(
            !output
                .diagnostics
                .iter()
                .any(|d| d.cause == ConstraintCause::PositionRecovery)
        );
        for joint in JOINTS {
            let state = output.reference[joint];
            let old = previous[joint];
            let dt = dt.as_secs_f64();
            let velocity = f64::from(parameters.maximum_velocity[joint]);
            let acceleration = f64::from(parameters.maximum_acceleration[joint]);
            let jerk = f64::from(parameters.maximum_jerk[joint]);
            assert!(state.velocity.abs() <= velocity + 1e-7);
            assert!(state.acceleration.abs() <= acceleration + 1e-7);
            assert!(state.jerk.abs() <= jerk + 1e-7);
            assert!((state.position - old.position).abs() <= velocity * dt + 1e-7);
            assert!((state.velocity - old.velocity).abs() <= acceleration * dt + 1e-7);
            assert!((state.acceleration - old.acceleration).abs() <= jerk * dt + 1e-7);
            let [min, max] = joints.position.head[joint];
            assert!((min..=max).contains(&output.commands[joint].position));
            observation.positions[joint] = output.commands[joint].position;
            observation.velocities[joint] = output.commands[joint].velocity;
        }
        if step == 10 {
            assert!(previous.yaw.velocity > 0.0);
            assert!(
                output.reference.yaw.velocity > 0.0,
                "reversal must brake before changing direction"
            );
        }
        previous = output.reference;
    }
    assert!(
        previous
            .into_iter()
            .all(|state| state.position.abs() < 1e-6 && state.velocity.abs() < 1e-6)
    );
}

#[test]
fn inactivity_and_damping_reseed_but_ordinary_tracking_does_not() {
    let (mut parameters, joints) = configuration();
    parameters.kp.yaw = 11.0;
    parameters.kd.yaw = 0.7;
    let mut controller = JointController::default();
    let target = JointTarget::Position(HeadJoints::fill(0.5));
    let start = controller
        .update(
            target,
            &observed(0.1, 0.3),
            &parameters,
            &joints,
            Time::zero(),
        )
        .unwrap();
    assert!(start.reseeded);
    assert_eq!((start.commands.yaw.kp, start.commands.yaw.kd), (11.0, 0.7));
    assert_eq!(start.commands.yaw.position, 0.1);
    assert_eq!(start.commands.yaw.velocity, 0.3);
    let next = controller
        .update(
            target,
            &observed(-0.2, 0.0),
            &parameters,
            &joints,
            Time::zero() + Duration::from_millis(20),
        )
        .unwrap();
    assert!(!next.reseeded);
    assert!(next.commands.yaw.position > 0.1);
    let resumed = controller
        .update(
            target,
            &observed(-0.1, -0.2),
            &parameters,
            &joints,
            Time::zero() + Duration::from_secs(1),
        )
        .unwrap();
    assert!(resumed.reseeded);
    assert_eq!(resumed.commands.yaw.position, -0.1);
    assert_eq!(resumed.commands.yaw.velocity, -0.2);
    let damping = controller
        .update(
            JointTarget::Damping,
            &observed(0.2, 0.4),
            &parameters,
            &joints,
            Time::zero() + Duration::from_millis(1020),
        )
        .unwrap();
    assert!(damping.progress.is_none());
    assert!(!damping.reseeded);
    for joint in JOINTS {
        let command = &damping.commands[joint];
        assert_eq!(
            (command.kp, command.velocity, command.torque),
            (0.0, 0.0, 0.0)
        );
        assert_eq!(command.kd, parameters.damping_kd[joint]);
    }
    let resumed = controller
        .update(
            target,
            &observed(0.3, 0.1),
            &parameters,
            &joints,
            Time::zero() + Duration::from_millis(1040),
        )
        .unwrap();
    assert!(resumed.reseeded);
    assert_eq!(resumed.commands.yaw.position, 0.3);
    // Robot/simulation clock reset also invalidates the previous trajectory.
    let reset = controller
        .update(
            target,
            &observed(-0.1, 0.0),
            &parameters,
            &joints,
            Time::zero(),
        )
        .unwrap();
    assert!(reset.reseeded);
    assert_eq!(reset.commands.yaw.position, -0.1);
}

#[test]
fn arrival_uses_constrained_goal_and_measured_velocity_with_hysteresis() {
    let (parameters, joints) = configuration();
    let mut controller = JointController::default();
    let target = JointTarget::Position(HeadJoints::fill(1.3));
    let goal = HeadJoints {
        yaw: joints.position.head.yaw[1],
        pitch: joints.position.head.pitch[1],
    };
    let mut observation = HeadObservation {
        positions: goal,
        velocities: HeadJoints::fill(-0.2),
    };
    let moving = controller
        .update(target, &observation, &parameters, &joints, Time::zero())
        .unwrap()
        .progress
        .unwrap();
    assert!(moving.constrained);
    assert_eq!(moving.effective_target, goal);
    assert!(!moving.target_reached);
    observation.velocities = HeadJoints::fill(0.0);
    let arrived = controller
        .update(
            target,
            &observation,
            &parameters,
            &joints,
            Time::zero() + Duration::from_millis(20),
        )
        .unwrap();
    assert!(arrived.progress.unwrap().target_reached);
    observation.positions = goal - HeadJoints::fill(0.025);
    assert!(
        controller
            .update(
                target,
                &observation,
                &parameters,
                &joints,
                Time::zero() + Duration::from_millis(40)
            )
            .unwrap()
            .progress
            .unwrap()
            .target_reached
    );
    observation.positions = goal - HeadJoints::fill(0.04);
    assert!(
        !controller
            .update(
                target,
                &observation,
                &parameters,
                &joints,
                Time::zero() + Duration::from_millis(60)
            )
            .unwrap()
            .progress
            .unwrap()
            .target_reached
    );
    // A different effective target must not inherit the previous arrival tolerance.
    observation.positions = goal;
    controller
        .update(
            target,
            &observation,
            &parameters,
            &joints,
            Time::zero() + Duration::from_millis(80),
        )
        .unwrap();
    let moved_goal = JointTarget::Position(goal - HeadJoints::fill(0.025));
    assert!(
        !controller
            .update(
                moved_goal,
                &observation,
                &parameters,
                &joints,
                Time::zero() + Duration::from_millis(100)
            )
            .unwrap()
            .progress
            .unwrap()
            .target_reached
    );
}

#[test]
fn infeasible_boundary_state_recovers_inside_position_limits() {
    let (parameters, joints) = configuration();
    for observation in [observed(0.79, 6.0), observed(2.0, 0.0)] {
        let mut controller = JointController::default();
        let output = controller
            .update(
                JointTarget::Position(HeadJoints::fill(0.0)),
                &observation,
                &parameters,
                &joints,
                Time::zero(),
            )
            .unwrap();
        assert!(
            output
                .diagnostics
                .iter()
                .any(|d| d.cause == ConstraintCause::PositionRecovery)
        );
        for joint in JOINTS {
            let [min, max] = joints.position.head[joint];
            assert!((min..=max).contains(&output.commands[joint].position));
        }
        assert_eq!(output.commands.pitch.velocity, 0.0);
    }
}

#[test]
fn tightening_limits_checks_the_existing_reference_before_emitting_it() {
    let (parameters, mut joints) = configuration();
    let mut controller = JointController::default();
    let observation = observed(0.5, 0.0);
    let target = JointTarget::Position(HeadJoints::fill(0.7));
    controller
        .update(target, &observation, &parameters, &joints, Time::zero())
        .unwrap();
    joints.position.head = HeadJoints::fill([-0.2, 0.2]);
    let output = controller
        .update(
            target,
            &observation,
            &parameters,
            &joints,
            Time::zero() + Duration::from_millis(20),
        )
        .unwrap();
    assert!(
        output
            .diagnostics
            .iter()
            .any(|d| d.cause == ConstraintCause::PositionRecovery)
    );
    assert!(
        output
            .commands
            .into_iter()
            .all(|command| command.position <= 0.2 && command.velocity == 0.0)
    );
}
