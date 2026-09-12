//! Keep log capture in its own test binary so parallel controller tests do not
//! race tracing callsite registration against the thread-local subscriber.
use std::sync::{Arc, Mutex};

use head_motion::{
    joint_control::{
        Constraint, ConstraintCause, ConstraintDiagnostic, HeadObservation, JointControlOutput,
    },
    logging::ConstraintLogger,
    parameters::Parameters,
};
use kinematics::joints::head::{HeadJoint, HeadJoints};
use ros_z::time::Time;
use tracing::Level;
use tracing_subscriber::{Layer, layer::SubscriberExt};
use types::motion_command::HeadMotion;
use types::motor_command::MotorCommand;

struct Capture(Arc<Mutex<Vec<Level>>>);
impl<S: tracing::Subscriber> Layer<S> for Capture {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        self.0.lock().unwrap().push(*event.metadata().level());
    }
}

#[test]
fn normal_reference_limits_are_debug_but_violations_remain_warnings() {
    let levels = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry().with(Capture(levels.clone()));
    tracing::subscriber::with_default(subscriber, || {
        let parameters: Parameters = json5::from_str(include_str!(
            "../../../../etc/parameters/base/head_motion.json5"
        ))
        .unwrap();
        let observation = HeadObservation {
            positions: HeadJoints::fill(0.0),
            velocities: HeadJoints::fill(0.0),
        };
        let mut logger = ConstraintLogger::default();
        for cause in [
            ConstraintCause::ReferenceAtBound,
            ConstraintCause::TargetClipped,
            ConstraintCause::MeasuredOutsideBounds,
            ConstraintCause::PositionRecovery,
        ] {
            let output = JointControlOutput {
                commands: HeadJoints::fill(MotorCommand::zeros()),
                reference: HeadJoints::default(),
                progress: None,
                diagnostics: vec![ConstraintDiagnostic {
                    joint: HeadJoint::Yaw,
                    constraint: Constraint::Position,
                    cause,
                    value: 1.0,
                    effective_value: 1.0,
                    bounds: [-1.0, 1.0],
                }],
                reseeded: false,
            };
            logger.log(
                &HeadMotion::ZeroAngles,
                &observation,
                &output,
                &parameters.joint_control,
                Time::zero(),
            );
        }
    });
    assert_eq!(
        *levels.lock().unwrap(),
        [Level::DEBUG, Level::WARN, Level::WARN, Level::WARN]
    );
}
