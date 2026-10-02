//! Robot-sized moving occluders shared by headless physics and the tuning viewer.
use bevy::prelude::*;
use mujoco_rs::prelude::{MjSpec, MjtGeom, SpecItem};

use crate::bevy_mujoco::MjcfObject;

pub(crate) const RADIUS: f32 = 0.22;
pub(crate) const HEIGHT: f32 = 0.85;
pub(crate) const COUNT: usize = 2;

const MAX_SPEED: f64 = 0.85;
const MAX_ACCELERATION: f64 = 1.2;
const ROBOT_CLEARANCE: f64 = 0.60;
const OPPONENT_CLEARANCE: f64 = 2.0 * RADIUS as f64 + 0.10;
const KICK_COOLDOWN: f64 = 4.5;

/// World-frame impulse applied by the caller through MuJoCo, never a position reset.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OpponentKick {
    pub ball_index: usize,
    pub impulse: [f64; 2],
}

pub(crate) struct ChallengeFrame {
    pub positions: [[f64; 3]; COUNT],
    pub kick: Option<OpponentKick>,
    pub contesting: bool,
}

/// A simple opponent policy, independent of the controlled robot's ball estimate.
///
/// The leading opponent approaches a foot-length stance between the closest ball
/// and our robot. Its body can occlude the ball before it kicks sideways. The
/// second opponent flanks the contest. Both use bounded speed and acceleration;
/// they remain mocap cylinders, not articulated or dynamically balanced robots.
/// Camera occlusion is evaluated by the normal synthetic perception path.
pub(crate) struct Challenge {
    centers: [[f64; 2]; COUNT],
    velocities: [[f64; 2]; COUNT],
    bounds: [f64; 2],
    side: f64,
    stance_distance: f64,
    maximum_kick_height: f64,
    ready_seconds: f64,
    cooldown: f64,
    kicks: u64,
    seed: u64,
}

impl Challenge {
    pub(crate) fn new(seed: u64, wall_half_extents: [f64; 2], ball_radius: f64) -> Self {
        let side = if seed.is_multiple_of(2) { 1.0 } else { -1.0 };
        let bounds = wall_half_extents.map(|extent| extent - f64::from(RADIUS) - 0.05);
        let mut result = Self {
            centers: [[2.5, 1.5 * side], [-0.8, -1.1 * side]],
            velocities: [[0.0; 2]; COUNT],
            bounds,
            side,
            stance_distance: f64::from(RADIUS) + ball_radius + 0.12,
            maximum_kick_height: ball_radius + 0.15,
            ready_seconds: 0.0,
            cooldown: 0.0,
            kicks: 0,
            seed,
        };
        for center in &mut result.centers {
            for axis in 0..2 {
                center[axis] = center[axis].clamp(-bounds[axis], bounds[axis]);
            }
        }
        result
    }

    pub(crate) fn positions(&self) -> [[f64; 3]; COUNT] {
        self.centers.map(|[x, y]| [x, y, f64::from(HEIGHT / 2.0)])
    }

    pub(crate) fn step(&mut self, dt: f64, robot: [f64; 2], balls: &[[f64; 3]]) -> ChallengeFrame {
        // Call once per physics step. A paused or invalid clock must not move bodies.
        if !dt.is_finite() || dt <= 0.0 {
            return ChallengeFrame {
                positions: self.positions(),
                kick: None,
                contesting: false,
            };
        }
        self.cooldown = (self.cooldown - dt).max(0.0);
        let nearest = balls.iter().enumerate().min_by(|(_, a), (_, b)| {
            distance([a[0], a[1]], robot).total_cmp(&distance([b[0], b[1]], robot))
        });
        let mut targets = self.centers;
        let mut stance = None;
        let mut contesting = false;
        if let Some((index, ball)) = nearest {
            let within_foot_height = ball[2] <= self.maximum_kick_height;
            let ball = [ball[0], ball[1]];
            let separation = distance(ball, robot);
            let direction = unit(sub(ball, robot));
            let lateral = [-direction[1] * self.side, direction[0] * self.side];
            targets[0] = sub(ball, scale(direction, self.stance_distance));
            targets[1] = add(ball, add(scale(lateral, 0.9), scale(direction, 0.25)));
            contesting = (1.0..=2.4).contains(&separation);
            stance = Some((index, ball, direction, targets[0], within_foot_height));
        }
        let old_centers = self.centers;
        for (index, target) in targets.iter_mut().enumerate() {
            for (axis, coordinate) in target.iter_mut().enumerate() {
                *coordinate = coordinate.clamp(-self.bounds[axis], self.bounds[axis]);
            }
            let mut desired = limit(scale(sub(*target, old_centers[index]), 2.0), MAX_SPEED);
            // Repel before contact, but never teleport a cylinder out of an overlap.
            for (other, clearance) in [
                (robot, ROBOT_CLEARANCE),
                (old_centers[1 - index], OPPONENT_CLEARANCE),
            ] {
                let away = sub(old_centers[index], other);
                let gap = norm(away);
                if gap < clearance + 0.25 {
                    desired = add(desired, scale(unit(away), 4.0 * (clearance + 0.25 - gap)));
                }
            }
            desired = limit(desired, MAX_SPEED);
            self.velocities[index] = add(
                self.velocities[index],
                limit(sub(desired, self.velocities[index]), MAX_ACCELERATION * dt),
            );
            let proposed = add(old_centers[index], scale(self.velocities[index], dt));
            let within_bounds = (0..2).all(|axis| proposed[axis].abs() <= self.bounds[axis]);
            let closing_on_robot = distance(proposed, robot) < ROBOT_CLEARANCE
                && distance(proposed, robot) < distance(old_centers[index], robot);
            let closing_on_opponent = distance(proposed, old_centers[1 - index])
                < OPPONENT_CLEARANCE
                && distance(proposed, old_centers[1 - index])
                    < distance(old_centers[index], old_centers[1 - index]);
            // Freeze an unsafe step instead of allowing infinite-mass mocap bodies
            // to walk through our robot. Motion can resume away from the contact.
            if within_bounds && !closing_on_robot && !closing_on_opponent {
                self.centers[index] = proposed;
            } else {
                self.velocities[index] = [0.0; 2];
            }
        }
        let mut kick = None;
        if let Some((ball_index, ball, direction, target, within_foot_height)) = stance {
            contesting &= self
                .centers
                .iter()
                .any(|&center| distance(center, ball) < 1.0);
            let ready = contesting
                && within_foot_height
                && self.cooldown == 0.0
                && distance(self.centers[0], target) < 0.10
                && distance(self.centers[0], ball) <= self.stance_distance + 0.10;
            self.ready_seconds = if ready { self.ready_seconds + dt } else { 0.0 };
            if self.ready_seconds >= 0.12 {
                // 1.9–2.9 N s produces fast passes with the physical 450 g ball.
                // Change the side/strength across challenges while remaining repeatable.
                let variation = (self.seed.wrapping_add(self.kicks * 7) % 11) as f64 / 10.0;
                let angle = self.side
                    * if self.kicks.is_multiple_of(2) {
                        1.05
                    } else {
                        -1.05
                    };
                let (sin, cos) = angle.sin_cos();
                kick = Some(OpponentKick {
                    ball_index,
                    impulse: scale(
                        [
                            cos * direction[0] - sin * direction[1],
                            sin * direction[0] + cos * direction[1],
                        ],
                        1.9 + variation,
                    ),
                });
                self.kicks += 1;
                self.ready_seconds = 0.0;
                self.cooldown = KICK_COOLDOWN;
            }
        } else {
            self.ready_seconds = 0.0;
        }
        ChallengeFrame {
            positions: self.positions(),
            kick,
            contesting,
        }
    }
}

fn add(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] + b[0], a[1] + b[1]]
}
fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn scale(a: [f64; 2], factor: f64) -> [f64; 2] {
    a.map(|value| value * factor)
}
fn norm(a: [f64; 2]) -> f64 {
    a[0].hypot(a[1])
}
fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    norm(sub(a, b))
}
fn unit(a: [f64; 2]) -> [f64; 2] {
    let length = norm(a);
    if length > 1e-9 {
        scale(a, 1.0 / length)
    } else {
        [1.0, 0.0]
    }
}
fn limit(a: [f64; 2], maximum: f64) -> [f64; 2] {
    let length = norm(a);
    if length > maximum {
        scale(a, maximum / length)
    } else {
        a
    }
}

pub(crate) fn object() -> MjcfObject {
    MjcfObject::from_factory(
        || {
            let mut spec = MjSpec::new();
            let body = spec.world_body_mut().add_body();
            body.set_name("opponent")
                .map_err(|error| error.to_string())?;
            body.with_mocap(true);
            body.add_geom()
                .with_type(MjtGeom::mjGEOM_CYLINDER)
                .with_size([f64::from(RADIUS), f64::from(HEIGHT / 2.0), 0.0])
                .with_friction([0.7, 0.005, 0.0001]);
            Ok(spec)
        },
        "opponent",
    )
    .with_mocap_body("opponent")
}

pub(crate) fn transform([x, y, z]: [f64; 3]) -> Transform {
    Transform::from_xyz(x as f32, z as f32, -y as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kick_requires_a_nearby_robot_and_a_reached_shielding_stance() {
        let robot = [2.0, 0.0];
        let ball = [3.8, 0.0, 0.105];
        let mut challenge = Challenge::new(42, [5.5, 4.0], 0.105);
        let mut kick_times = Vec::new();
        for step in 0..10_000 {
            let frame = challenge.step(0.002, robot, &[ball]);
            if let Some(kick) = frame.kick {
                kick_times.push(step as f64 * 0.002);
                let opponent = frame.positions[0];
                assert_eq!(kick.ball_index, 0);
                assert!(opponent[0] > robot[0] && opponent[0] < ball[0]);
                assert!(opponent[1].abs() < 0.10, "body must shield ball");
                assert!(distance([opponent[0], opponent[1]], [ball[0], ball[1]]) < 0.55);
                assert!((1.9..=2.901).contains(&norm(kick.impulse)));
                assert!(
                    kick.impulse[1].abs() > 1.5,
                    "kick should cross the view quickly"
                );
            }
        }
        assert!(
            kick_times.len() >= 2,
            "a reachable contest must produce kicks"
        );
        assert!(kick_times[0] > 1.0, "opponent must first approach the ball");
        assert!(
            kick_times
                .windows(2)
                .all(|times| times[1] - times[0] >= KICK_COOLDOWN)
        );

        let mut distant = Challenge::new(42, [5.5, 4.0], 0.105);
        for _ in 0..10_000 {
            assert!(distant.step(0.002, [0.0, 0.0], &[ball]).kick.is_none());
        }
    }

    #[test]
    fn airborne_ball_is_followed_but_never_kicked_by_a_foot() {
        let mut challenge = Challenge::new(42, [5.5, 4.0], 0.105);
        for _ in 0..10_000 {
            assert!(
                challenge
                    .step(0.002, [2.0, 0.0], &[[3.8, 0.0, 0.8]])
                    .kick
                    .is_none()
            );
        }
        assert!(distance(challenge.centers[0], [3.8, 0.0]) < 0.55);
    }

    #[test]
    fn motion_is_bounded_and_does_not_step_into_robot_or_wall() {
        let robot = [2.0, 0.0];
        let mut challenge = Challenge::new(42, [5.5, 4.0], 0.105);
        let mut previous = challenge.positions();
        for step in 0..20_000 {
            // Abruptly switch the target, including beyond a wall. Only targets,
            // never simulated bodies, may jump between those positions.
            let ball = if step < 10_000 {
                [2.1, 0.0, 0.105]
            } else {
                [8.0, 6.0, 0.105]
            };
            let frame = challenge.step(0.002, robot, &[ball]);
            for (index, p) in frame.positions.iter().enumerate() {
                assert!(
                    distance([p[0], p[1]], [previous[index][0], previous[index][1]])
                        <= MAX_SPEED * 0.002 + 1e-10
                );
                assert!(distance([p[0], p[1]], robot) >= ROBOT_CLEARANCE - 1e-10);
                assert!(p[0].abs() <= challenge.bounds[0] && p[1].abs() <= challenge.bounds[1]);
            }
            assert!(
                distance(challenge.centers[0], challenge.centers[1]) >= OPPONENT_CLEARANCE - 0.002
            );
            previous = frame.positions;
        }
    }

    #[test]
    fn no_balls_means_no_kicks_and_invalid_clock_does_not_move_opponents() {
        let mut challenge = Challenge::new(43, [5.5, 4.0], 0.105);
        let initial = challenge.positions();
        for dt in [0.0, -0.1, f64::NAN, f64::INFINITY] {
            assert_eq!(challenge.step(dt, [0.0, 0.0], &[]).positions, initial);
        }
        for _ in 0..1000 {
            let frame = challenge.step(0.002, [0.0, 0.0], &[]);
            assert!(frame.kick.is_none());
            assert!(!frame.contesting);
            assert_eq!(frame.positions, initial);
        }
    }

    #[test]
    fn seeded_challenges_repeat_and_approaches_mirror() {
        let mut even = Challenge::new(42, [5.5, 4.0], 0.105);
        let mut repeat = Challenge::new(42, [5.5, 4.0], 0.105);
        let mut odd = Challenge::new(43, [5.5, 4.0], 0.105);
        for _ in 0..1000 {
            let a = even.step(0.002, [0.0, 0.0], &[[3.8, 0.65, 0.105]]);
            let b = repeat.step(0.002, [0.0, 0.0], &[[3.8, 0.65, 0.105]]);
            let c = odd.step(0.002, [0.0, 0.0], &[[3.8, -0.65, 0.105]]);
            assert_eq!(a.positions, b.positions);
            for (a, c) in a.positions.iter().zip(c.positions) {
                assert!((a[0] - c[0]).abs() < 1e-10);
                assert!((a[1] + c[1]).abs() < 1e-10);
            }
        }
    }
}
