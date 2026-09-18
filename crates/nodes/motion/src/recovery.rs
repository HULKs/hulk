use super::*;
use motion_inference::config::Policy;
use ros_z::time::Time;
use types::{
    fall_detection::{FallDetection, Posture},
    motion_command::ImageRegion,
    motion_execution::MotionPhase,
};

#[derive(Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub(super) struct RecoveryParameters {
    pub maximum_duration: Duration,
    pub endpoint_grace: Duration,
    pub minimum_fast_duration: Duration,
    pub settling_duration: Duration,
    pub settling_timeout: Duration,
}
impl RecoveryParameters {
    pub fn validate(&self) -> std::result::Result<(), String> {
        if [
            self.maximum_duration,
            self.endpoint_grace,
            self.minimum_fast_duration,
            self.settling_duration,
            self.settling_timeout,
        ]
        .into_iter()
        .any(|v| v.is_zero())
            || self.minimum_fast_duration >= self.maximum_duration
            || self.settling_duration >= self.settling_timeout
        {
            return Err("invalid recovery durations".into());
        }
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct Recovery {
    stage: Stage,
}
#[derive(Default)]
enum Stage {
    #[default]
    Inactive,
    Normal,
    Recovering(Attempt),
    Settling {
        requested_at: Time,
        started_at: Option<Time>,
    },
}
struct Attempt {
    fast: bool,
    requested_at: Time,
    execution: Option<PolicyExecution>,
    endpoint_at: Option<Time>,
}
impl Attempt {
    fn is_complete(&self, fall: &FallDetection, now: Time, p: &RecoveryParameters) -> Result<bool> {
        ensure!(
            now >= self.requested_at,
            "clock moved backwards during recovery"
        );
        ensure!(
            now.duration_since(self.requested_at) < p.maximum_duration,
            "recovery timed out"
        );
        if let Some(endpoint) = self.endpoint_at {
            ensure!(
                now.duration_since(endpoint) < p.endpoint_grace || fall.ready_for_walk,
                "get-up reached its endpoint without a stable walking posture"
            );
        }
        let completed = self.execution.is_some_and(|e| {
            if self.fast {
                now.duration_since(e.started_at) >= p.minimum_fast_duration
            } else {
                e.progress.is_some_and(|progress| progress >= 1.0)
            }
        });
        Ok(completed && fall.ready_for_walk && fall.posture == Posture::Upright)
    }
}

impl Recovery {
    pub fn reset(&mut self) {
        self.stage = Stage::Inactive;
    }

    pub fn phase(&self) -> MotionPhase {
        match &self.stage {
            Stage::Inactive => MotionPhase::Damping,
            Stage::Normal => MotionPhase::Normal,
            Stage::Recovering(a) => MotionPhase::Recovering { fast: a.fast },
            Stage::Settling { .. } => MotionPhase::Settling,
        }
    }

    pub fn execution(&self) -> Option<PolicyExecution> {
        match &self.stage {
            Stage::Recovering(a) => a.execution,
            _ => None,
        }
    }

    /// Called only for accepted output, so policy time cannot start during the mode handshake.
    pub fn observe(&mut self, execution: PolicyExecution, now: Time) {
        match &mut self.stage {
            Stage::Recovering(a)
                if matches!(execution.policy, Policy::SlowGetUp | Policy::FastGetUp) =>
            {
                a.execution = Some(execution);
                if execution.progress.is_some_and(|progress| progress >= 1.0) {
                    a.endpoint_at.get_or_insert(now);
                }
            }
            Stage::Settling { started_at, .. } if execution.policy == Policy::Walk => {
                started_at.get_or_insert(execution.started_at);
            }
            _ => {}
        }
    }

    pub fn allows_output(&self, fall: &FallDetection) -> bool {
        matches!(self.stage, Stage::Recovering(_)) || fall.posture == Posture::Upright
    }

    pub fn select(
        &mut self,
        command: &MotionCommand,
        command_time: Time,
        fall: &FallDetection,
        now: Time,
        p: &Parameters,
    ) -> Result<MotionPlan> {
        if let Stage::Recovering(attempt) = &self.stage {
            if !attempt.is_complete(fall, now, &p.recovery)? {
                return Ok(MotionPlan::GetUp {
                    command: GetUpCommand { fast: attempt.fast },
                });
            }
            self.stage = Stage::Settling {
                requested_at: now,
                started_at: None,
            };
        }
        if !self.allows_output(fall) {
            return Ok(self.when_fallen(command, fall, now));
        }
        if matches!(self.stage, Stage::Inactive) {
            if !fall.ready_for_walk {
                return Ok(MotionPlan::Damping);
            }
            self.stage = Stage::Settling {
                requested_at: now,
                started_at: None,
            };
        }
        if let Stage::Settling {
            requested_at,
            started_at,
        } = self.stage
        {
            if let Some(start) = started_at {
                ensure!(
                    now >= start && now.duration_since(start) < p.recovery.settling_timeout,
                    "walking handover timed out"
                );
            }
            // Before the first Walk output, hardware's acknowledgement deadline and
            // the inference deadline bound activation independently.
            let settled = started_at
                .is_some_and(|start| now.duration_since(start) >= p.recovery.settling_duration);
            if !settled
                || !fall.ready_for_walk
                || command_time <= requested_at
                || matches!(command, MotionCommand::StandUp { .. })
            {
                return Ok(zero_walk(command));
            }
            self.stage = Stage::Normal;
        }
        // A repeated get-up request cannot restart a completed recovery while upright.
        if matches!(command, MotionCommand::StandUp { .. }) {
            return Ok(zero_walk(command));
        }
        MotionPlan::from_motion_command(command, &p.walking)
    }

    fn when_fallen(
        &mut self,
        command: &MotionCommand,
        fall: &FallDetection,
        now: Time,
    ) -> MotionPlan {
        self.reset();
        if let MotionCommand::StandUp { fast } = command
            && fall.posture == Posture::Fallen
        {
            self.stage = Stage::Recovering(Attempt {
                fast: *fast,
                requested_at: now,
                execution: None,
                endpoint_at: None,
            });
            MotionPlan::GetUp {
                command: GetUpCommand { fast: *fast },
            }
        } else {
            MotionPlan::Damping
        }
    }
}

fn zero_walk(command: &MotionCommand) -> MotionPlan {
    MotionPlan::Walk {
        head_motion: command.head_motion().unwrap_or(HeadMotion::Center {
            image_region_target: ImageRegion::Top,
        }),
        command: WalkCommand::stand(),
    }
}
