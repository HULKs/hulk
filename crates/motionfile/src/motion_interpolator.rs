use std::fmt::Debug;
use std::time::Duration;

use crate::spline_interpolator::SplineInterpolator;
use crate::MotionFileFrame;
use crate::{condition::ConditionEnum, Condition, MotionFile};
use color_eyre::eyre::Context;
use color_eyre::{Report, Result};
use splines::{Interpolate, Interpolation, Key};
use types::SensorData;

pub struct MotionInterpolator<T>
where
    T: Debug + Interpolate<f32>,
{
    items: Vec<MotionItem<T>>,
    index: usize,
}

#[derive(Debug)]
pub enum MotionItem<T>
where
    T: Debug + Interpolate<f32>,
{
    Spline(SplineInterpolator<T>),
    Condition(ConditionEnum),
}

impl<T: Debug + Interpolate<f32>> MotionItem<T> {
    pub fn is_finished(&self) -> bool {
        match self {
            MotionItem::Spline(spline) => spline.is_finished(),
            MotionItem::Condition(condition) => condition.is_finished(),
        }
    }
}

impl<T: Debug + Interpolate<f32>> MotionInterpolator<T> {
    pub fn is_waiting_for_condition(&self) -> bool {
        !self.is_finished() && matches!(self.items[self.index], MotionItem::Condition(_))
    }

    fn get_prior_spline(&self) -> Option<&SplineInterpolator<T>> {
        self.items[(0..self.index)]
            .iter()
            .rev()
            .find_map(|item| match item {
                MotionItem::Spline(spline) => Some(spline),
                _ => None,
            })
    }

    fn get_next_spline(&self) -> Option<&SplineInterpolator<T>> {
        self.items[self.index..].iter().find_map(|item| match item {
            MotionItem::Spline(spline) => Some(spline),
            _ => None,
        })
    }

    pub fn advance_by(&mut self, time_step: Duration, sensor_data: &SensorData) {
        self.update(sensor_data);
        let item = &mut self.items[self.index];

        if let MotionItem::Spline(interpolator) = item {
            interpolator.advance_by(time_step);
        }

        if item.is_finished() && self.index < self.items.len() - 1 {
            self.index += 1;
        }
    }

    pub fn is_finished(&self) -> bool {
        self.index == self.items.len() - 1 && self.items.last().unwrap().is_finished()
    }

    pub fn value(&self) -> Result<T> {
        match &self.items[self.index] {
            MotionItem::Spline(spline) => spline
                .value()
                .wrap_err("failed to compute spline in MotionFileInterpolator"),
            MotionItem::Condition(_) => self
                .get_prior_spline()
                .map(|spline| spline.end_position())
                .or_else(|| self.get_next_spline().map(|spline| spline.start_position()))
                .ok_or_else(|| Report::msg("no splines in motion file")),
        }
    }

    pub fn reset(&mut self) {
        for item in self.items[0..=self.index].iter_mut() {
            match item {
                MotionItem::Spline(spline) => spline.reset(),
                MotionItem::Condition(condition) => condition.reset(),
            }
        }
        self.index = 0;
    }

    fn update(&mut self, sensor_data: &SensorData) {
        self.items.iter_mut().for_each(|item| {
            if let MotionItem::Condition(condition) = item {
                condition.update(sensor_data)
            }
        })
    }
}

impl<T: Debug + Interpolate<f32>> TryFrom<MotionFile<T>> for MotionInterpolator<T> {
    type Error = Report;

    fn try_from(motion_file: MotionFile<T>) -> Result<Self> {
        let mut current_time = Duration::ZERO;
        let mut current_spline_frames = vec![Key::new(
            current_time,
            motion_file.initial_positions,
            Interpolation::Linear,
        )];

        let mut motion_items = Vec::new();

        for frame in motion_file.frames {
            match frame {
                MotionFileFrame::Joints {
                    duration,
                    positions,
                } => {
                    current_time += duration;
                    current_spline_frames.push(Key::new(
                        current_time,
                        positions,
                        Interpolation::Linear,
                    ));
                }
                MotionFileFrame::Condition(condition) => {
                    motion_items.push(MotionItem::Spline(SplineInterpolator::try_new(
                        current_spline_frames.clone(),
                    )?));
                    let last = current_spline_frames.pop().unwrap();
                    current_spline_frames.clear();
                    current_spline_frames.push(last);
                    motion_items.push(MotionItem::Condition(condition));
                }
            }
        }
        if current_spline_frames.len() > 1 {
            motion_items.push(MotionItem::Spline(SplineInterpolator::try_new(
                current_spline_frames,
            )?));
        }

        Ok(Self {
            items: motion_items,
            index: 0,
        })
    }
}
