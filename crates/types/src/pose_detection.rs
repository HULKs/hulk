use std::ops::Index;

use crate::bounding_box::BoundingBox;
use color_eyre::Result;
use coordinate_systems::Pixel;
use linear_algebra::{point, Point2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, SerializeHierarchy)]
pub struct Keypoint {
    pub point: Point2<Pixel>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, SerializeHierarchy)]
pub struct Keypoints {
    pub left_eye: Keypoint,
    pub right_eye: Keypoint,
    pub nose: Keypoint,
    pub left_ear: Keypoint,
    pub right_ear: Keypoint,
    pub left_shoulder: Keypoint,
    pub right_shoulder: Keypoint,
    pub left_hand: Keypoint,
    pub right_hand: Keypoint,
    pub left_elbow: Keypoint,
    pub right_elbow: Keypoint,
    pub left_hip: Keypoint,
    pub right_hip: Keypoint,
    pub left_knee: Keypoint,
    pub right_knee: Keypoint,
    pub left_foot: Keypoint,
    pub right_foot: Keypoint,
}

impl Keypoints {
    pub fn try_new(keypoints_slice: &[f32], x_scale: f32, y_scale: f32) -> Option<Self> {
        let mut keypoints_iter = keypoints_slice.chunks(3).map(|keypoint_chunk| Keypoint {
            point: point![keypoint_chunk[0] * x_scale, keypoint_chunk[1] * y_scale],
            confidence: keypoint_chunk[2],
        });

        Some(Self {
            left_eye: keypoints_iter.next()?,
            right_eye: keypoints_iter.next()?,
            nose: keypoints_iter.next()?,
            left_ear: keypoints_iter.next()?,
            right_ear: keypoints_iter.next()?,
            left_shoulder: keypoints_iter.next()?,
            right_shoulder: keypoints_iter.next()?,
            left_hand: keypoints_iter.next()?,
            right_hand: keypoints_iter.next()?,
            left_elbow: keypoints_iter.next()?,
            right_elbow: keypoints_iter.next()?,
            left_hip: keypoints_iter.next()?,
            right_hip: keypoints_iter.next()?,
            left_knee: keypoints_iter.next()?,
            right_knee: keypoints_iter.next()?,
            left_foot: keypoints_iter.next()?,
            right_foot: keypoints_iter.next()?,
        })
    }
}
impl Index<usize> for Keypoints {
    fn index(&self, index: usize) -> &Keypoint {
        assert!((0..=16).contains(&index));
        match index {
            0 => &self.left_eye,
            1 => &self.right_eye,
            2 => &self.nose,
            3 => &self.left_ear,
            4 => &self.right_ear,
            5 => &self.left_shoulder,
            6 => &self.right_shoulder,
            9 => &self.left_hand,
            10 => &self.right_hand,
            7 => &self.left_hand,
            8 => &self.right_hand,
            11 => &self.left_hip,
            12 => &self.right_hip,
            13 => &self.left_knee,
            14 => &self.right_knee,
            15 => &self.left_foot,
            16 => &self.right_foot,
            _ => unreachable!(),
        }
    }
    type Output = Keypoint;
}
impl From<Keypoints> for [Keypoint; 17] {
    fn from(keypoints: Keypoints) -> Self {
        [
            keypoints.left_eye,
            keypoints.right_eye,
            keypoints.nose,
            keypoints.left_ear,
            keypoints.right_ear,
            keypoints.left_shoulder,
            keypoints.right_shoulder,
            keypoints.left_hand,
            keypoints.right_hand,
            keypoints.left_elbow,
            keypoints.right_elbow,
            keypoints.left_hip,
            keypoints.right_hip,
            keypoints.left_knee,
            keypoints.right_knee,
            keypoints.left_foot,
            keypoints.right_foot,
        ]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, SerializeHierarchy)]
pub struct HumanPose {
    pub bounding_box: BoundingBox,
    pub keypoints: Keypoints,
}

impl HumanPose {
    pub fn new(bounding_box: BoundingBox, keypoints: Keypoints) -> HumanPose {
        Self {
            bounding_box,
            keypoints,
        }
    }
}
