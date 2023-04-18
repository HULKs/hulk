use std::time::Duration;

use crate::spline_interpolator::SplineInterpolator;
use color_eyre::{eyre::Context, Report, Result};
use motionfile::{Condition, MotionFile, condition::ConditionEnum};
use types::{Joints, SensorData};

pub struct MotionInterpolator {
    items: Vec<MotionItem>,
    index: usize,
}

pub enum MotionItem {
    Spline(SplineInterpolator),
    Condition(ConditionEnum),
}

impl MotionItem {
    pub fn is_finished(&self) -> bool {
        match self {
            MotionItem::Spline(spline) => spline.is_finished(),
            MotionItem::Condition(condition) => condition.is_finished(),
        }
    }
}

impl MotionInterpolator {
    fn get_prior_spline(&self) -> Option<&SplineInterpolator> {
        self.items[(0..self.index)]
            .iter()
            .rev()
            .find_map(|item| match item {
                MotionItem::Spline(spline) => Some(spline),
                _ => None,
            })
    }

    fn get_next_spline(&self) -> Option<&SplineInterpolator> {
        self.items[self.index..].iter().find_map(|item| match item {
            MotionItem::Spline(spline) => Some(spline),
            _ => None,
        })
    }

    pub fn advance_by(&mut self, time_step: Duration) {
        let item = &mut self.items[self.index];

        if let MotionItem::Spline(interpolator) = item {
            interpolator.advance_by(time_step);
        }

        if item.is_finished() && self.index < self.items.len() - 1 {
            self.index += 1
        }
    }

    pub fn is_finished(&self) -> bool {
        self.index == self.items.len() - 1 && self.items.last().unwrap().is_finished()
    }

    pub fn value(&self) -> Result<Joints> {
        match &self.items[self.index] {
            MotionItem::Spline(spline) => spline
                .value()
                .wrap_err("failed to compute spline in MotionFileInterpolator"),
            MotionItem::Condition(condition) => condition
                .value()
                .or_else(|| self.get_prior_spline().map(|spline| spline.end_position()))
                .or_else(|| self.get_next_spline().map(|spline| spline.start_position()))
                .ok_or_else(|| Report::msg("no splines in motion file")),
        }
    }

    pub fn reset(&mut self) {
        for item in self.items.iter_mut() {
            match item {
                MotionItem::Spline(spline) => spline.reset(),
                MotionItem::Condition(condition) => condition.reset(),
            }
        }
    }

    pub fn update(&mut self, sensor_data: &SensorData) {
        self.items.iter_mut().for_each(|item| {
            if let MotionItem::Condition(condition) = item {
                condition.update(sensor_data)
            }
        })
    }
}

impl TryFrom<MotionFile> for MotionInterpolator {
    type Error = Report;

    fn try_from(motion_file: MotionFile) -> Result<Self> {
        Ok(Self {
            items: vec![MotionItem::Spline(
                SplineInterpolator::try_from(motion_file)
                    .wrap_err("failed to create spline interpolator from motion file")?,
            )],
            index: 0,
        })
    }
}
