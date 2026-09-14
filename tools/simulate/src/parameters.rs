use std::sync::Arc;

use bevy::prelude::*;
use ros_z::parameter::{NodeParameters, ParameterSubscription};
use serde::{Deserialize, Serialize};
use types::field_dimensions::FieldDimensions;

#[derive(Debug, Clone, Serialize, Deserialize, ros_z::Message)]
#[message(name = "simulate::SimulatorParameters")]
#[serde(deny_unknown_fields)]
pub struct SimulatorParameters {
    pub field_dimensions: FieldDimensions,
    pub ball: BallParameters,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
#[message(name = "simulate::BallParameters")]
#[serde(deny_unknown_fields)]
pub struct BallParameters {
    pub mass: f32,
    pub joint_damping: f32,
    pub joint_friction_loss: f32,
    /// MuJoCo geom friction: sliding, torsional, rolling.
    pub friction: [f32; 3],
    /// MuJoCo contact solver reference parameters.
    pub solref: [f32; 2],
    /// MuJoCo contact solver impedance parameters.
    pub solimp: [f32; 5],
}

impl SimulatorParameters {
    pub fn validate(parameters: &Self) -> Result<(), String> {
        let dimensions = &parameters.field_dimensions;
        let positive = [
            ("ball_radius", dimensions.ball_radius),
            ("length", dimensions.length),
            ("width", dimensions.width),
            ("line_width", dimensions.line_width),
            ("penalty_marker_size", dimensions.penalty_marker_size),
            ("goal_box_area_length", dimensions.goal_box_area_length),
            ("goal_box_area_width", dimensions.goal_box_area_width),
            ("penalty_area_length", dimensions.penalty_area_length),
            ("penalty_area_width", dimensions.penalty_area_width),
            (
                "penalty_marker_distance",
                dimensions.penalty_marker_distance,
            ),
            ("center_circle_diameter", dimensions.center_circle_diameter),
            ("goal_inner_width", dimensions.goal_inner_width),
            ("goal_post_diameter", dimensions.goal_post_diameter),
            ("goal_depth", dimensions.goal_depth),
        ];
        for (name, value) in positive {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!(
                    "field_dimensions.{name} must be finite and positive"
                ));
            }
        }
        for (name, value) in [
            ("border_strip_width", dimensions.border_strip_width),
            ("corner_arc_radius", dimensions.corner_arc_radius),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!(
                    "field_dimensions.{name} must be finite and non-negative"
                ));
            }
        }

        let half_length = dimensions.length / 2.0;
        if dimensions.goal_box_area_length > half_length
            || dimensions.penalty_area_length > half_length
            || dimensions.penalty_marker_distance > half_length
        {
            return Err("field lengthwise markings must fit inside one field half".to_string());
        }
        if dimensions.goal_box_area_width > dimensions.width
            || dimensions.penalty_area_width > dimensions.width
            || dimensions.goal_inner_width + 2.0 * dimensions.goal_post_diameter > dimensions.width
        {
            return Err("field widthwise markings and goal must fit inside the field".to_string());
        }
        if dimensions.goal_box_area_length > dimensions.penalty_area_length
            || dimensions.goal_box_area_width > dimensions.penalty_area_width
        {
            return Err("goal box must fit inside the penalty area".to_string());
        }
        if dimensions.center_circle_diameter > dimensions.length.min(dimensions.width)
            || dimensions.corner_arc_radius > dimensions.length.min(dimensions.width) / 2.0
        {
            return Err("circular field markings must fit inside the field".to_string());
        }

        parameters.ball.validate()?;

        Ok(())
    }
}

impl BallParameters {
    fn validate(&self) -> Result<(), String> {
        if !self.mass.is_finite() || self.mass <= 0.0 {
            return Err("ball.mass must be finite and positive".to_string());
        }
        for (name, value) in [
            ("joint_damping", self.joint_damping),
            ("joint_friction_loss", self.joint_friction_loss),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!("ball.{name} must be finite and non-negative"));
            }
        }
        if self
            .friction
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err("ball.friction values must be finite and non-negative".to_string());
        }

        let [reference, damping] = self.solref;
        if !reference.is_finite()
            || !damping.is_finite()
            || !((reference > 0.0 && damping > 0.0) || (reference < 0.0 && damping <= 0.0))
        {
            return Err(
                "ball.solref must use either positive standard format or negative direct format"
                    .to_string(),
            );
        }

        let [initial, final_, width, midpoint, power] = self.solimp;
        if !self.solimp.iter().all(|value| value.is_finite())
            || !(0.0..1.0).contains(&initial)
            || !(0.0..1.0).contains(&final_)
            || width <= 0.0
            || !(0.0..=1.0).contains(&midpoint)
            || power < 1.0
        {
            return Err(
                "ball.solimp must contain valid impedance, width, midpoint, and power values"
                    .to_string(),
            );
        }

        Ok(())
    }
}

#[derive(Resource)]
pub struct CurrentSimulatorParameters {
    pub revision: u64,
    pub parameters: Arc<SimulatorParameters>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct SimulatorParameterSyncSet;

#[derive(Resource)]
struct SimulatorParameterAdapter {
    _binding: NodeParameters<SimulatorParameters>,
    receiver: ParameterSubscription<SimulatorParameters>,
}

pub struct SimulatorParametersPlugin {
    parameters: NodeParameters<SimulatorParameters>,
}

impl SimulatorParametersPlugin {
    pub fn new(parameters: NodeParameters<SimulatorParameters>) -> Self {
        Self { parameters }
    }
}

impl Plugin for SimulatorParametersPlugin {
    fn build(&self, app: &mut App) {
        let snapshot = self.parameters.snapshot();
        app.insert_resource(CurrentSimulatorParameters {
            revision: snapshot.revision,
            parameters: snapshot.typed.clone(),
        })
        .insert_resource(SimulatorParameterAdapter {
            _binding: self.parameters.clone(),
            receiver: self.parameters.subscribe(),
        })
        .configure_sets(PreUpdate, SimulatorParameterSyncSet)
        .add_systems(
            PreUpdate,
            synchronize_simulator_parameters.in_set(SimulatorParameterSyncSet),
        );
    }
}

fn synchronize_simulator_parameters(
    mut adapter: ResMut<SimulatorParameterAdapter>,
    mut current: ResMut<CurrentSimulatorParameters>,
) {
    if !adapter.receiver.has_changed().unwrap_or(false) {
        return;
    }

    let snapshot = adapter.receiver.borrow_and_update().clone();
    current.revision = snapshot.revision;
    current.parameters = snapshot.typed.clone();
}
