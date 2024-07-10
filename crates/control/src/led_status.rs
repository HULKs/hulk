use std::time::{Duration, SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use serde::{Deserialize, Serialize};
use types::{
    ball_detection::Ball,
    color::Rgb,
    cycle_time::CycleTime,
    filtered_whistle::FilteredWhistle,
    led::{Ear, Eye, Leds},
    messages::IncomingMessage,
    primary_state::PrimaryState,
    roles::Role,
    sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct LedStatus {
    blink_state: bool,
    last_blink_toggle: SystemTime,
    last_ball_top: SystemTime,
    last_ball_bottom: SystemTime,
    last_game_controller_message: Option<SystemTime>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    primary_state: Input<PrimaryState, "primary_state">,
    cycle_time: Input<CycleTime, "cycle_time">,
    filtered_whistle: Input<FilteredWhistle, "filtered_whistle">,
    role: Input<Role, "role">,
    is_own_referee_ready_pose_detected: Input<bool, "is_referee_ready_pose_detected">,
    did_detect_any_referee_this_cycle: Input<bool, "did_detect_any_referee_this_cycle">,

    balls_bottom: PerceptionInput<Option<Vec<Ball>>, "VisionBottom", "balls?">,
    balls_top: PerceptionInput<Option<Vec<Ball>>, "VisionTop", "balls?">,
    network_message: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,
    sensor_data: Input<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub leds: MainOutput<Leds>,
}

struct BallPercepts {
    at_least_one_ball_top: bool,
    at_least_one_ball_bottom: bool,
    last_ball_top_too_old: bool,
    last_ball_bottom_too_old: bool,
}

impl LedStatus {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            blink_state: true,
            last_blink_toggle: UNIX_EPOCH,
            last_ball_top: UNIX_EPOCH,
            last_ball_bottom: UNIX_EPOCH,
            last_game_controller_message: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        if context
            .cycle_time
            .start_time
            .duration_since(self.last_blink_toggle)
            .unwrap()
            >= Duration::from_millis(500)
        {
            self.last_blink_toggle = context.cycle_time.start_time;
            self.blink_state = !self.blink_state;
        }

        let chest = match context.primary_state {
            PrimaryState::Unstiff => match self.blink_state {
                true => Rgb::BLUE,
                false => Rgb::BLACK,
            },
            PrimaryState::Initial => Rgb::BLACK,
            PrimaryState::Ready => Rgb::BLUE,
            PrimaryState::Set => Rgb::YELLOW,
            PrimaryState::Playing => Rgb::GREEN,
            PrimaryState::Penalized => Rgb::RED,
            PrimaryState::Finished => Rgb::BLACK,
            PrimaryState::Calibration => Rgb::PURPLE,
            PrimaryState::Standby => Rgb::TURQUOISE,
        };

        let at_least_one_ball_top =
            context
                .balls_top
                .persistent
                .values()
                .rev()
                .flatten()
                .any(|balls| {
                    if let Some(balls) = balls {
                        !balls.is_empty()
                    } else {
                        false
                    }
                });
        let newer_ball_top = context
            .balls_top
            .persistent
            .values()
            .rev()
            .flatten()
            .find_map(|balls| {
                if balls.is_some() {
                    Some(context.cycle_time.start_time)
                } else {
                    None
                }
            });
        if let Some(newer_ball_top) = newer_ball_top {
            self.last_ball_top = newer_ball_top;
        }
        let last_ball_top_too_old = context
            .cycle_time
            .start_time
            .duration_since(self.last_ball_top)
            .unwrap()
            > Duration::from_secs(1);

        let at_least_one_ball_bottom = context
            .balls_bottom
            .persistent
            .values()
            .rev()
            .flatten()
            .any(|balls| {
                if let Some(balls) = balls {
                    !balls.is_empty()
                } else {
                    false
                }
            });
        let newer_ball_bottom = context
            .balls_bottom
            .persistent
            .values()
            .rev()
            .flatten()
            .find_map(|balls| {
                if balls.is_some() {
                    Some(context.cycle_time.start_time)
                } else {
                    None
                }
            });
        if let Some(newer_ball_bottom) = newer_ball_bottom {
            self.last_ball_bottom = newer_ball_bottom;
        }
        let last_ball_bottom_too_old = context
            .cycle_time
            .start_time
            .duration_since(self.last_ball_bottom)
            .unwrap()
            > Duration::from_secs(1);

        let ball_percepts = BallPercepts {
            at_least_one_ball_top,
            at_least_one_ball_bottom,
            last_ball_top_too_old,
            last_ball_bottom_too_old,
        };

        let (left_eye, right_eye) = Self::get_eyes(
            context.cycle_time.start_time,
            context.primary_state,
            context.role,
            ball_percepts,
            *context.is_own_referee_ready_pose_detected,
            *context.did_detect_any_referee_this_cycle,
        );

        if let Some(latest_game_controller_message_time) = context
            .network_message
            .persistent
            .iter()
            .rev()
            .find_map(|(timestamp, messages)| {
                messages
                    .iter()
                    .flatten()
                    .any(|&message| matches!(message, IncomingMessage::GameController(..)))
                    .then_some(timestamp)
            })
        {
            self.last_game_controller_message = Some(*latest_game_controller_message_time);
        };

        let ears = Self::get_ears(
            context.filtered_whistle.is_detected,
            context.cycle_time.start_time,
            self.last_game_controller_message,
            self.blink_state,
            context
                .sensor_data
                .temperature_sensors
                .into_iter()
                .fold(0.0, f32::max),
        );

        let leds = Leds {
            left_ear: ears,
            right_ear: ears,
            chest,
            left_foot: Rgb::GREEN,
            right_foot: Rgb::GREEN,
            left_eye,
            right_eye,
        };

        Ok(MainOutputs { leds: leds.into() })
    }

    fn get_ears(
        filter_whistle_detected: bool,
        cycle_start_time: SystemTime,
        last_game_controller_message: Option<SystemTime>,
        blink_state: bool,
        current_maximum_temperature: f32,
    ) -> Ear {
        let mut ear = if last_game_controller_message.is_some_and(|timestamp| {
            cycle_start_time
                .duration_since(timestamp)
                .expect("time ran backwards")
                > Duration::from_millis(5000)
        }) {
            if blink_state {
                Ear::full_ears(1.0)
            } else {
                Ear::full_ears(0.0)
            }
        } else {
            // values, at which the stiffness gets automatically reduced by the motors
            const TEMPERATURE_LEVEL_ONE: f32 = 76.0;
            const TEMPERATURE_LEVEL_TWO: f32 = 80.0;
            const TEMPERATURE_LEVEL_THREE: f32 = 90.0;

            let ear_fraction = if current_maximum_temperature > TEMPERATURE_LEVEL_ONE {
                0.33
            } else if current_maximum_temperature > TEMPERATURE_LEVEL_TWO {
                0.66
            } else if current_maximum_temperature > TEMPERATURE_LEVEL_THREE {
                1.0
            } else {
                0.0
            };
            Ear::percentage_ears(1.0, ear_fraction)
        };

        if filter_whistle_detected {
            ear = ear.invert();
        }

        ear
    }

    fn get_eyes(
        cycle_start_time: SystemTime,
        primary_state: &PrimaryState,
        role: &Role,
        ball_percepts: BallPercepts,
        is_own_referee_ready_pose_detected: bool,
        did_detect_any_referee_this_cycle: bool,
    ) -> (Eye, Eye) {
        match primary_state {
            PrimaryState::Unstiff => {
                let rainbow_eye = Self::get_rainbow_eye(cycle_start_time);
                (rainbow_eye, rainbow_eye)
            }
            _ => {
                let ball_background_color = if ball_percepts.at_least_one_ball_top
                    || ball_percepts.at_least_one_ball_bottom
                {
                    Some(Rgb::GREEN)
                } else {
                    None
                };
                let ball_color_top = if ball_percepts.last_ball_top_too_old {
                    Some(Rgb::RED)
                } else {
                    None
                };
                let ball_color_bottom = if ball_percepts.last_ball_bottom_too_old {
                    Some(Rgb::RED)
                } else {
                    None
                };
                let right_color = match role {
                    Role::DefenderLeft
                    | Role::DefenderRight
                    | Role::MidfielderLeft
                    | Role::MidfielderRight => Rgb::BLUE,
                    Role::Keeper | Role::ReplacementKeeper => Rgb::YELLOW,
                    Role::Loser => Rgb::BLACK,
                    Role::Searcher => Rgb::WHITE,
                    Role::Striker => Rgb::RED,
                    Role::StrikerSupporter => Rgb::TURQUOISE,
                };
                let filtered_referee_ready_color = if is_own_referee_ready_pose_detected {
                    Some(Rgb::YELLOW)
                } else {
                    None
                };
                let referee_ready_percept_color = if did_detect_any_referee_this_cycle {
                    Some(Rgb::PURPLE)
                } else {
                    None
                };
                (
                    Eye {
                        color_at_0: ball_color_top
                            .or(filtered_referee_ready_color)
                            .or(referee_ready_percept_color)
                            .or(ball_background_color)
                            .unwrap_or(Rgb::BLACK),
                        color_at_45: ball_color_top
                            .or(filtered_referee_ready_color)
                            .or(referee_ready_percept_color)
                            .or(ball_background_color)
                            .unwrap_or(Rgb::BLACK),
                        color_at_90: ball_background_color
                            .or(filtered_referee_ready_color)
                            .or(referee_ready_percept_color)
                            .unwrap_or(Rgb::BLACK),
                        color_at_135: ball_color_bottom
                            .or(filtered_referee_ready_color)
                            .or(referee_ready_percept_color)
                            .or(ball_background_color)
                            .unwrap_or(Rgb::BLACK),
                        color_at_180: ball_color_bottom
                            .or(filtered_referee_ready_color)
                            .or(referee_ready_percept_color)
                            .or(ball_background_color)
                            .unwrap_or(Rgb::BLACK),
                        color_at_225: ball_color_bottom
                            .or(filtered_referee_ready_color)
                            .or(referee_ready_percept_color)
                            .or(ball_background_color)
                            .unwrap_or(Rgb::BLACK),
                        color_at_270: ball_background_color
                            .or(filtered_referee_ready_color)
                            .or(referee_ready_percept_color)
                            .unwrap_or(Rgb::BLACK),
                        color_at_315: ball_color_top
                            .or(filtered_referee_ready_color)
                            .or(referee_ready_percept_color)
                            .or(ball_background_color)
                            .unwrap_or(Rgb::BLACK),
                    },
                    Eye::from(right_color),
                )
            }
        }
    }

    fn get_rainbow_eye(cycle_start_time: SystemTime) -> Eye {
        let seconds = cycle_start_time
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        let fraction = 1.0 / 8.0;
        Eye {
            color_at_0: Self::interval_ratio_to_rainbow_color({
                let offsetted_seconds = seconds - (0.0 * fraction);
                (offsetted_seconds - offsetted_seconds.floor()) as f32
            }),
            color_at_45: Self::interval_ratio_to_rainbow_color({
                let offsetted_seconds = seconds - (1.0 * fraction);
                (offsetted_seconds - offsetted_seconds.floor()) as f32
            }),
            color_at_90: Self::interval_ratio_to_rainbow_color({
                let offsetted_seconds = seconds - (2.0 * fraction);
                (offsetted_seconds - offsetted_seconds.floor()) as f32
            }),
            color_at_135: Self::interval_ratio_to_rainbow_color({
                let offsetted_seconds = seconds - (3.0 * fraction);
                (offsetted_seconds - offsetted_seconds.floor()) as f32
            }),
            color_at_180: Self::interval_ratio_to_rainbow_color({
                let offsetted_seconds = seconds - (4.0 * fraction);
                (offsetted_seconds - offsetted_seconds.floor()) as f32
            }),
            color_at_225: Self::interval_ratio_to_rainbow_color({
                let offsetted_seconds = seconds - (5.0 * fraction);
                (offsetted_seconds - offsetted_seconds.floor()) as f32
            }),
            color_at_270: Self::interval_ratio_to_rainbow_color({
                let offsetted_seconds = seconds - (6.0 * fraction);
                (offsetted_seconds - offsetted_seconds.floor()) as f32
            }),
            color_at_315: Self::interval_ratio_to_rainbow_color({
                let offsetted_seconds = seconds - (7.0 * fraction);
                (offsetted_seconds - offsetted_seconds.floor()) as f32
            }),
        }
    }

    /// interval_ratio in [0.0, 1.0)
    pub fn interval_ratio_to_rainbow_color(interval_ratio: f32) -> Rgb {
        let interval_ratio_over_6 = interval_ratio * 6.0;
        let fraction = ((interval_ratio_over_6 - interval_ratio_over_6.floor()) * 255.0) as u8;
        let section = interval_ratio_over_6 as u8;
        match section {
            0 | 6 => Rgb::new(255, fraction, 0),
            1 => Rgb::new(255 - fraction, 255, 0),
            2 => Rgb::new(0, 255, fraction),
            3 => Rgb::new(0, 255 - fraction, 255),
            4 => Rgb::new(fraction, 0, 255),
            5 => Rgb::new(255, 0, 255 - fraction),
            _ => unreachable!(),
        }
    }
}
