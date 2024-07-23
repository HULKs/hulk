use bevy::prelude::*;

use types::ball_position::SimulatorBallState;

#[derive(Resource)]
pub struct BallResource {
    pub state: Option<SimulatorBallState>,
    pub friction_coefficient: f32,
}

impl Default for BallResource {
    fn default() -> Self {
        Self {
            state: None,
            friction_coefficient: 0.98,
        }
    }
}

pub fn move_ball(mut ball: ResMut<BallResource>, time: Res<Time>) {
    let friction = ball.friction_coefficient;

    if let Some(state) = ball.state.as_mut() {
        state.position += state.velocity * time.delta_seconds();
        state.velocity *= friction;
    }
}
