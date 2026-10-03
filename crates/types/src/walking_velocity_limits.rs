use coordinate_systems::Ground;
use linear_algebra::{Vector2, vector};
use serde::{Deserialize, Serialize};

pub const WALKING_VELOCITY_LIMITS_TOPIC: &str = "walking_velocity_limits";

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct WalkingVelocityLimits {
    pub forward_velocity_limits: [f32; 2],
    pub lateral_velocity_limit: f32,
    pub angular_velocity_limit: f32,
}

impl WalkingVelocityLimits {
    pub fn validate(self) -> Result<(), String> {
        let [minimum, maximum] = self.forward_velocity_limits;
        if !minimum.is_finite() || !maximum.is_finite() || minimum > 0.0 || maximum < 0.0 {
            return Err(format!(
                "forward velocity limits must be finite and contain zero, got [{minimum}, {maximum}]"
            ));
        }
        for (name, limit) in [
            ("lateral", self.lateral_velocity_limit),
            ("angular", self.angular_velocity_limit),
        ] {
            if !limit.is_finite() || limit <= 0.0 {
                return Err(format!(
                    "{name} velocity limit must be finite and positive, got {limit}"
                ));
            }
        }
        Ok(())
    }

    pub fn scale_forward_axis(self, axis: f32) -> f32 {
        let axis = finite_axis(axis);
        if axis < 0.0 {
            -axis * self.forward_velocity_limits[0]
        } else {
            axis * self.forward_velocity_limits[1]
        }
    }

    pub fn scale_lateral_axis(self, axis: f32) -> f32 {
        finite_axis(axis) * self.lateral_velocity_limit
    }

    pub fn scale_angular_axis(self, axis: f32) -> f32 {
        finite_axis(axis) * self.angular_velocity_limit
    }

    pub fn clamp_command(
        self,
        velocity: Vector2<Ground>,
        angular_velocity: f32,
    ) -> (Vector2<Ground>, f32) {
        (
            vector![
                finite_or_zero(velocity.x()).clamp(
                    self.forward_velocity_limits[0],
                    self.forward_velocity_limits[1]
                ),
                finite_or_zero(velocity.y())
                    .clamp(-self.lateral_velocity_limit, self.lateral_velocity_limit),
            ],
            finite_or_zero(angular_velocity)
                .clamp(-self.angular_velocity_limit, self.angular_velocity_limit),
        )
    }
}

fn finite_axis(value: f32) -> f32 {
    finite_or_zero(value).clamp(-1.0, 1.0)
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}
