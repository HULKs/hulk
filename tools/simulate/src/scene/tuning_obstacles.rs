//! Robot-sized moving occluders shared by headless physics and the tuning viewer.
use bevy::prelude::*;
use mujoco_rs::prelude::{MjSpec, MjtGeom, SpecItem};
use types::ball_filter_tuning::OpponentParameters;

use crate::bevy_mujoco::MjcfObject;

pub(crate) const HEIGHT: f32 = 0.85;

const MAX_SPEED: f64 = 0.85;
const MAX_ACCELERATION: f64 = 1.2;
const ROBOT_CLEARANCE: f64 = 0.60;
const KICK_COOLDOWN: f64 = 4.5;

/// World-frame impulse applied by the caller through MuJoCo, never a position reset.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OpponentKick {
    pub ball_index: usize,
    pub impulse: [f64; 2],
}

pub(crate) struct ChallengeFrame {
    pub positions: Vec<[f64; 3]>,
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
    centers: Vec<[f64; 2]>,
    velocities: Vec<[f64; 2]>,
    opponent_clearance: f64,
    robot_clearance: f64,
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
    #[cfg(test)]
    pub(crate) fn new(seed: u64, wall_half_extents: [f64; 2], ball_radius: f64) -> Self {
        Self::with_opponents(
            seed,
            wall_half_extents,
            ball_radius,
            OpponentParameters::default(),
        )
        .expect("test field fits the default opponents")
    }

    pub(crate) fn with_opponents(
        seed: u64,
        wall_half_extents: [f64; 2],
        ball_radius: f64,
        parameters: OpponentParameters,
    ) -> Result<Self, String> {
        if !parameters.is_valid() || !ball_radius.is_finite() || ball_radius <= 0.0 {
            return Err("invalid opponent configuration or ball radius".into());
        }
        let side = if seed.is_multiple_of(2) { 1.0 } else { -1.0 };
        let radius = f64::from(parameters.width / 2.0);
        let bounds = wall_half_extents.map(|extent| extent - radius - 0.05);
        if bounds
            .iter()
            .any(|bound| !bound.is_finite() || *bound <= 0.0)
        {
            return Err("field is too small to contain opponents with wall clearance".into());
        }
        let centers = (0..parameters.count)
            .map(|index| match index {
                0 => [2.5, 1.5 * side],
                1 => [-0.8, -1.1 * side],
                _ => {
                    let angle =
                        f64::from(index) * std::f64::consts::TAU / f64::from(parameters.count);
                    [0.8 * bounds[0] * angle.cos(), 0.8 * bounds[1] * angle.sin()]
                }
            })
            .collect();
        let mut result = Self {
            centers,
            velocities: vec![[0.0; 2]; parameters.count as usize],
            opponent_clearance: 2.0 * radius + 0.10,
            robot_clearance: (radius + 0.38).max(ROBOT_CLEARANCE),
            bounds,
            side,
            stance_distance: radius + ball_radius + 0.12,
            maximum_kick_height: ball_radius + 0.15,
            ready_seconds: 0.0,
            cooldown: 0.0,
            kicks: 0,
            seed,
        };
        result.avoid_initial_balls(&[])?;
        Ok(result)
    }

    /// Place bodies before physics starts; never call this to resolve live contacts.
    /// Preserve safe starts and move unsafe ones to the nearest deterministic grid
    /// candidate with clearance from the robot, previously placed opponents and balls.
    pub(crate) fn avoid_initial_balls(&mut self, balls: &[[f64; 2]]) -> Result<(), String> {
        if balls
            .iter()
            .flatten()
            .any(|coordinate| !coordinate.is_finite())
        {
            return Err("initial ball positions must be finite".into());
        }
        let mut placed = Vec::with_capacity(self.centers.len());
        let ball_clearance = self.stance_distance - 0.07;
        // Limit grid size even for unusually large fields. Standard fields sample
        // at half an opponent's separation, including both boundary coordinates.
        let spacing = (self.opponent_clearance / 2.0).max(0.1);
        let steps = self
            .bounds
            .map(|bound| ((2.0 * bound / spacing).ceil() as usize).clamp(1, 256));
        let bounds = self.bounds;
        for (index, &preferred) in self.centers.iter().enumerate() {
            let preferred = std::array::from_fn(|axis| {
                preferred[axis].clamp(-self.bounds[axis], self.bounds[axis])
            });
            let safe = |point: [f64; 2]| {
                distance(point, [0.0, 0.0]) >= self.robot_clearance
                    && placed
                        .iter()
                        .all(|&other| distance(point, other) >= self.opponent_clearance)
                    && balls
                        .iter()
                        .all(|&ball| distance(point, ball) >= ball_clearance)
            };
            let selected = if safe(preferred) {
                Some(preferred)
            } else {
                (0..=steps[0])
                    .flat_map(|x| {
                        (0..=steps[1]).map(move |y| {
                            [
                                bounds[0] * (2.0 * x as f64 / steps[0] as f64 - 1.0),
                                bounds[1] * (2.0 * y as f64 / steps[1] as f64 - 1.0),
                            ]
                        })
                    })
                    .filter(|&point| safe(point))
                    .min_by(|&a, &b| distance(a, preferred).total_cmp(&distance(b, preferred)))
            };
            let Some(selected) = selected else {
                return Err(format!(
                    "cannot place opponent {index} with robot, ball and wall clearance"
                ));
            };
            placed.push(selected);
        }
        self.centers = placed;
        Ok(())
    }

    pub(crate) fn positions(&self) -> Vec<[f64; 3]> {
        self.centers
            .iter()
            .map(|&[x, y]| [x, y, f64::from(HEIGHT / 2.0)])
            .collect()
    }

    pub(crate) fn step(&mut self, dt: f64, robot: [f64; 2], balls: &[[f64; 3]]) -> ChallengeFrame {
        // Call once per physics step. A paused or invalid clock must not move bodies.
        if !dt.is_finite() || dt <= 0.0 || self.centers.is_empty() {
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
        let mut targets = self.centers.clone();
        let mut stance = None;
        let mut contesting = false;
        if let Some((index, ball)) = nearest {
            let within_foot_height = ball[2] <= self.maximum_kick_height;
            let ball = [ball[0], ball[1]];
            let separation = distance(ball, robot);
            let direction = unit(sub(ball, robot));
            let lateral = [-direction[1] * self.side, direction[0] * self.side];
            targets[0] = sub(ball, scale(direction, self.stance_distance));
            for (index, target) in targets.iter_mut().enumerate().skip(1) {
                let flank = if index.is_multiple_of(2) { -1.0 } else { 1.0 };
                let spacing =
                    0.9_f64.max(self.opponent_clearance + 0.15) * (1.0 + ((index - 1) / 2) as f64);
                *target = add(
                    ball,
                    add(scale(lateral, flank * spacing), scale(direction, 0.25)),
                );
            }
            contesting = (1.0..=2.4).contains(&separation);
            stance = Some((index, ball, direction, targets[0], within_foot_height));
        }
        let old_centers = self.centers.clone();
        for (index, target) in targets.iter_mut().enumerate() {
            for (axis, coordinate) in target.iter_mut().enumerate() {
                *coordinate = coordinate.clamp(-self.bounds[axis], self.bounds[axis]);
            }
            let mut desired = limit(scale(sub(*target, old_centers[index]), 2.0), MAX_SPEED);
            // Repel before contact, but never teleport a cylinder out of an overlap.
            for (other, clearance) in std::iter::once((robot, self.robot_clearance)).chain(
                old_centers
                    .iter()
                    .enumerate()
                    .filter(|(other, _)| *other != index)
                    .map(|(_, &other)| (other, self.opponent_clearance)),
            ) {
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
            let closing_on_robot = distance(proposed, robot) < self.robot_clearance
                && distance(proposed, robot) < distance(old_centers[index], robot);
            let closing_on_opponent = old_centers.iter().enumerate().any(|(other, &center)| {
                other != index
                    && distance(proposed, center) < self.opponent_clearance
                    && distance(proposed, center) < distance(old_centers[index], center)
            });
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

pub(crate) fn object(radius: f32) -> MjcfObject {
    MjcfObject::from_factory(
        move || {
            let mut spec = MjSpec::new();
            let body = spec.world_body_mut().add_body();
            body.set_name("opponent")
                .map_err(|error| error.to_string())?;
            body.with_mocap(true);
            body.add_geom()
                .with_type(MjtGeom::mjGEOM_CYLINDER)
                .with_size([f64::from(radius), f64::from(HEIGHT / 2.0), 0.0])
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
    fn configurable_counts_and_widths_keep_motion_bounded() {
        for count in [0, 1, 2, 8] {
            for width in [0.1, 0.44, 1.2] {
                let mut challenge = Challenge::with_opponents(
                    42,
                    [5.5, 4.0],
                    0.105,
                    OpponentParameters { count, width },
                )
                .unwrap();
                assert_eq!(challenge.positions().len(), count as usize);
                assert!(
                    (challenge.stance_distance - (f64::from(width / 2.0) + 0.225)).abs() < 1e-9
                );
                let mut previous = challenge.positions();
                for _ in 0..1000 {
                    let frame = challenge.step(0.002, [0.0, 0.0], &[[3.8, 0.65, 0.105]]);
                    assert_eq!(frame.positions.len(), count as usize);
                    if count == 0 {
                        assert!(frame.kick.is_none());
                    }
                    for (point, old) in frame.positions.iter().zip(&previous) {
                        assert!(
                            distance([point[0], point[1]], [old[0], old[1]])
                                <= MAX_SPEED * 0.002 + 1e-10
                        );
                        assert!(
                            point[0].abs() <= challenge.bounds[0]
                                && point[1].abs() <= challenge.bounds[1]
                        );
                    }
                    previous = frame.positions;
                }
            }
        }
    }

    #[test]
    fn initial_placement_has_clearance_for_every_supported_count_and_width() {
        for seed in [42, 43] {
            for count in 0..=8 {
                for width in [0.1, 0.44, 1.2] {
                    let mut challenge = Challenge::with_opponents(
                        seed,
                        [5.5, 4.0],
                        0.105,
                        OpponentParameters { count, width },
                    )
                    .unwrap();
                    let side = if seed % 2 == 0 { 1.0 } else { -1.0 };
                    let balls = [[3.8, 0.65 * side], [-2.4, 1.4 * side], [1.0, -2.0 * side]];
                    challenge.avoid_initial_balls(&balls).unwrap();
                    assert_eq!(challenge.centers.len(), count as usize);
                    for (index, &point) in challenge.centers.iter().enumerate() {
                        assert!(distance(point, [0.0, 0.0]) >= challenge.robot_clearance);
                        assert!((0..2).all(|axis| point[axis].abs() <= challenge.bounds[axis]));
                        assert!(
                            balls
                                .iter()
                                .all(|&ball| distance(point, ball)
                                    >= challenge.stance_distance - 0.07)
                        );
                        assert!(
                            challenge.centers[..index].iter().all(
                                |&other| distance(point, other) >= challenge.opponent_clearance
                            )
                        );
                    }
                    let initial = challenge.positions();
                    challenge.avoid_initial_balls(&balls).unwrap();
                    assert_eq!(
                        initial,
                        challenge.positions(),
                        "safe positions should remain unchanged"
                    );
                }
            }
        }
    }

    #[test]
    fn invalid_bounds_and_impossible_packing_return_errors() {
        for bounds in [[0.1, 0.1], [f64::NAN, 4.0], [5.5, f64::INFINITY]] {
            assert!(
                Challenge::with_opponents(42, bounds, 0.105, OpponentParameters::default())
                    .is_err()
            );
        }
        assert!(
            Challenge::with_opponents(
                42,
                [0.7, 0.7],
                0.105,
                OpponentParameters {
                    count: 8,
                    width: 1.2
                }
            )
            .is_err()
        );
        assert!(
            Challenge::with_opponents(
                42,
                [5.5, 4.0],
                0.105,
                OpponentParameters {
                    count: 9,
                    width: 0.44
                }
            )
            .is_err()
        );
        let mut challenge = Challenge::new(42, [5.5, 4.0], 0.105);
        let before = challenge.positions();
        assert!(challenge.avoid_initial_balls(&[[f64::NAN, 0.0]]).is_err());
        assert_eq!(before, challenge.positions());
    }

    #[test]
    fn safe_default_starts_are_preserved() {
        for (seed, side) in [(42, 1.0), (43, -1.0)] {
            let challenge = Challenge::new(seed, [5.5, 4.0], 0.105);
            assert_eq!(challenge.centers, [[2.5, 1.5 * side], [-0.8, -1.1 * side]]);
        }
    }

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
                distance(challenge.centers[0], challenge.centers[1])
                    >= challenge.opponent_clearance - 0.002
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
