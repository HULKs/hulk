use std::{
    ops::{Add, Div, Index, IndexMut, Mul, Sub},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy)]
pub enum HeadJointsName {
    Yaw,
    Pitch,
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, SerializeHierarchy,
)]
#[serialize_hierarchy(bound = "T: SerializeHierarchy + Serialize, for<'de> T: Deserialize<'de>")]
pub struct HeadJoints<T> {
    pub yaw: T,
    pub pitch: T,
}

impl<T> HeadJoints<T>
where
    T: Clone,
{
    pub fn fill(value: T) -> Self {
        Self {
            yaw: value.clone(),
            pitch: value,
        }
    }
}

impl<T> IntoIterator for HeadJoints<T> {
    type Item = T;

    type IntoIter = std::array::IntoIter<T, 2>;

    fn into_iter(self) -> Self::IntoIter {
        [self.yaw, self.pitch].into_iter()
    }
}

impl<T> Add for HeadJoints<T>
where
    T: Add,
{
    type Output = HeadJoints<<T as Add>::Output>;

    fn add(self, right: Self) -> Self::Output {
        Self::Output {
            yaw: self.yaw + right.yaw,
            pitch: self.pitch + right.pitch,
        }
    }
}

impl<T> Sub for HeadJoints<T>
where
    T: Sub,
{
    type Output = HeadJoints<<T as Sub>::Output>;

    fn sub(self, right: Self) -> Self::Output {
        Self::Output {
            yaw: self.yaw - right.yaw,
            pitch: self.pitch - right.pitch,
        }
    }
}

impl Mul<f32> for HeadJoints<f32> {
    type Output = HeadJoints<f32>;

    fn mul(self, right: f32) -> Self::Output {
        Self::Output {
            yaw: self.yaw * right,
            pitch: self.pitch * right,
        }
    }
}

impl Div<f32> for HeadJoints<f32> {
    type Output = HeadJoints<f32>;

    fn div(self, right: f32) -> Self::Output {
        Self::Output {
            yaw: self.yaw / right,
            pitch: self.pitch / right,
        }
    }
}

impl Div<HeadJoints<f32>> for HeadJoints<f32> {
    type Output = HeadJoints<Duration>;

    fn div(self, right: HeadJoints<f32>) -> Self::Output {
        Self::Output {
            yaw: Duration::from_secs_f32((self.yaw / right.yaw).abs()),
            pitch: Duration::from_secs_f32((self.pitch / right.pitch).abs()),
        }
    }
}

impl HeadJoints<f32> {
    pub fn mirrored(self) -> Self {
        Self {
            yaw: -self.yaw,
            pitch: self.pitch,
        }
    }
}

impl<T> Index<HeadJointsName> for HeadJoints<T> {
    type Output = T;

    fn index(&self, index: HeadJointsName) -> &Self::Output {
        match index {
            HeadJointsName::Yaw => &self.yaw,
            HeadJointsName::Pitch => &self.pitch,
        }
    }
}

impl<T> IndexMut<HeadJointsName> for HeadJoints<T> {
    fn index_mut(&mut self, index: HeadJointsName) -> &mut Self::Output {
        match index {
            HeadJointsName::Yaw => &mut self.yaw,
            HeadJointsName::Pitch => &mut self.pitch,
        }
    }
}
