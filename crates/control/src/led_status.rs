use std::time::{Duration, SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use types::{Ball, CycleTime, Eye, FilteredWhistle, Leds, MotionCommand, PrimaryState, Rgb, Ear};

pub struct LedStatus {
    blink_state: bool,
    last_blink_toggle: SystemTime,
    last_ball_data_top: SystemTime,
    last_ball_data_bottom: SystemTime,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub primary_state: Input<PrimaryState, "primary_state">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub filtered_whistle: Input<FilteredWhistle, "filtered_whistle">,
    pub motion_command: Input<MotionCommand, "motion_command">,

    pub balls_bottom: PerceptionInput<Option<Vec<Ball>>, "VisionBottom", "balls?">,
    pub balls_top: PerceptionInput<Option<Vec<Ball>>, "VisionTop", "balls?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub leds: MainOutput<Leds>,
}

impl LedStatus {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            blink_state: true,
            last_blink_toggle: UNIX_EPOCH,
            last_ball_data_top: UNIX_EPOCH,
            last_ball_data_bottom: UNIX_EPOCH,
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
        };

        let at_least_one_ball_data_top =
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
        let newer_ball_data_top = context
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
        if let Some(newer_ball_data_top) = newer_ball_data_top {
            self.last_ball_data_top = newer_ball_data_top;
        }
        let last_ball_data_top_too_old = context
            .cycle_time
            .start_time
            .duration_since(self.last_ball_data_top)
            .unwrap()
            > Duration::from_secs(1);

        let at_least_one_ball_data_bottom = context
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
        let newer_ball_data_bottom = context
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
        if let Some(newer_ball_data_bottom) = newer_ball_data_bottom {
            self.last_ball_data_bottom = newer_ball_data_bottom;
        }
        let last_ball_data_bottom_too_old = context
            .cycle_time
            .start_time
            .duration_since(self.last_ball_data_bottom)
            .unwrap()
            > Duration::from_secs(1);

        let (left_eye, right_eye) = Self::get_eyes(
            context.cycle_time.start_time,
            context.primary_state,
            at_least_one_ball_data_top,
            at_least_one_ball_data_bottom,
            last_ball_data_top_too_old,
            last_ball_data_bottom_too_old,
        );

        let mut ears = if context.filtered_whistle.is_detected {
            1.0
        } else {
            0.0
        }
        .into();

        if let MotionCommand::FallProtection { .. } = context.motion_command {
            ears = Ear::every_second(1.0);
        }

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

    fn get_eyes(
        cycle_start_time: SystemTime,
        primary_state: &PrimaryState,
        at_least_one_ball_data_top: bool,
        at_least_one_ball_data_bottom: bool,
        last_ball_data_top_too_old: bool,
        last_ball_data_bottom_too_old: bool,
    ) -> (Eye, Eye) {
        match primary_state {
            PrimaryState::Unstiff => {
                let rainbow_eye = Self::get_rainbow_eye(cycle_start_time);
                (rainbow_eye, rainbow_eye)
            }
            _ => {
                let ball_background_color =
                    if at_least_one_ball_data_top || at_least_one_ball_data_bottom {
                        Some(Rgb::GREEN)
                    } else {
                        None
                    };
                let ball_color_top = if last_ball_data_top_too_old {
                    Some(Rgb::RED)
                } else {
                    None
                };
                let ball_color_bottom = if last_ball_data_bottom_too_old {
                    Some(Rgb::RED)
                } else {
                    None
                };
                (
                    Eye {
                        color_at_0: ball_color_top
                            .unwrap_or_else(|| ball_background_color.unwrap_or(Rgb::BLACK)),
                        color_at_45: ball_color_top
                            .unwrap_or_else(|| ball_background_color.unwrap_or(Rgb::BLACK)),
                        color_at_90: ball_background_color.unwrap_or(Rgb::BLACK),
                        color_at_135: ball_color_bottom
                            .unwrap_or_else(|| ball_background_color.unwrap_or(Rgb::BLACK)),
                        color_at_180: ball_color_bottom
                            .unwrap_or_else(|| ball_background_color.unwrap_or(Rgb::BLACK)),
                        color_at_225: ball_color_bottom
                            .unwrap_or_else(|| ball_background_color.unwrap_or(Rgb::BLACK)),
                        color_at_270: ball_background_color.unwrap_or(Rgb::BLACK),
                        color_at_315: ball_color_top
                            .unwrap_or_else(|| ball_background_color.unwrap_or(Rgb::BLACK)),
                    },
                    Eye::default(),
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
