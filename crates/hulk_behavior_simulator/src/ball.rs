use bevy::prelude::*;

use types::ball_position::SimulatorBallState;

#[derive(Resource, Default)]
pub struct BallResource {
    pub state: Option<SimulatorBallState>,
}

pub fn move_ball(mut ball: ResMut<BallResource>, time: Res<Time>) {
    if let Some(ball) = ball.state.as_mut() {
        ball.position += ball.velocity * time.delta_seconds();
        ball.velocity *= 0.98;
    }
}
