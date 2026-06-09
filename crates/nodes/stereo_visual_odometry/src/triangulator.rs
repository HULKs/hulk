use color_eyre::{
    Result,
    eyre::{bail, ensure},
};
use kornia_algebra::{Mat3AF32, Vec2F32, Vec3AF32};
use projection::intrinsic::Intrinsic;
use ros2::sensor_msgs::camera_info::CameraInfo;

use crate::feature_extractor::{CurrentLeft, CurrentRight, FrameFeatures, FrameKeypoints, Matches};

const CAMERA_EPSILON: f64 = 1e-9;
const MAX_VERTICAL_DISPARITY_PX: f32 = 3.0;

pub struct StereoTriangulator {
    left_size: (u32, u32),
    right_size: (u32, u32),
    fx: f32,
    fy: f32,
    cx: f32,
    cy: f32,
    baseline: f32,
    intrinsics: Mat3AF32,
}

pub struct StereoPoint {
    pub left_index: usize,
    pub position: Vec3AF32,
}

impl StereoTriangulator {
    pub fn new(left: &CameraInfo, right: &CameraInfo) -> Result<Self> {
        check_camera_info(left, "left")?;
        check_camera_info(right, "right")?;
        ensure_same_intrinsics(left, right)?;

        let baseline = projection_x(left) - projection_x(right);
        ensure!(baseline.abs() > CAMERA_EPSILON, "stereo baseline is zero");

        let intrinsics = Intrinsic::from(left);
        let fx = intrinsics.focals.x;
        let fy = intrinsics.focals.y;
        let cx = intrinsics.optical_center.x();
        let cy = intrinsics.optical_center.y();
        let baseline = baseline.abs() as f32;
        ensure!(
            fx.is_finite()
                && fy.is_finite()
                && cx.is_finite()
                && cy.is_finite()
                && baseline.is_finite(),
            "stereo camera calibration contains non-finite values"
        );

        Ok(Self {
            left_size: (left.width, left.height),
            right_size: (right.width, right.height),
            fx,
            fy,
            cx,
            cy,
            baseline,
            intrinsics: Mat3AF32::from_cols(
                Vec3AF32::new(fx, 0.0, 0.0),
                Vec3AF32::new(0.0, fy, 0.0),
                Vec3AF32::new(cx, cy, 1.0),
            ),
        })
    }

    pub fn triangulate_into(
        &self,
        left: FrameFeatures<'_, CurrentLeft>,
        right: FrameKeypoints<'_, CurrentRight>,
        matches: Matches<'_, CurrentLeft, CurrentRight>,
        output: &mut Vec<StereoPoint>,
    ) {
        output.clear();

        for (left_index, right_index, _score) in matches.left_to_right() {
            let Some(left_keypoint) = left.keypoint(left_index) else {
                continue;
            };
            let Some(right_keypoint) = right.keypoint(right_index) else {
                continue;
            };
            if !left.is_valid(left_index) {
                continue;
            }

            let left_pixel = self.left_pixel(left_keypoint);
            let right_pixel = self.right_pixel(right_keypoint);
            if let Some(position) = self.triangulate_point(left_pixel, right_pixel) {
                output.push(StereoPoint {
                    left_index,
                    position,
                });
            }
        }
    }

    pub fn left_pixel(&self, keypoint: [f32; 2]) -> Vec2F32 {
        denormalize(keypoint, self.left_size)
    }

    pub fn intrinsics_f32(&self) -> &Mat3AF32 {
        &self.intrinsics
    }

    fn right_pixel(&self, keypoint: [f32; 2]) -> Vec2F32 {
        denormalize(keypoint, self.right_size)
    }

    fn triangulate_point(&self, left: Vec2F32, right: Vec2F32) -> Option<Vec3AF32> {
        let disparity = left.x - right.x;
        if !left.y.is_finite()
            || !right.y.is_finite()
            || !disparity.is_finite()
            || disparity <= 0.0
            || (left.y - right.y).abs() > MAX_VERTICAL_DISPARITY_PX
        {
            println!("{left:?} {right:?}");
            return None;
        }

        let z = self.fx * self.baseline / disparity;
        let x = (left.x - self.cx) * z / self.fx;
        let y = (left.y - self.cy) * z / self.fy;
        (z.is_finite() && z > 0.0 && x.is_finite() && y.is_finite())
            .then_some(Vec3AF32::new(x, y, z))
    }
}

fn denormalize([x, y]: [f32; 2], (width, height): (u32, u32)) -> Vec2F32 {
    let width = width as f32;
    let height = height as f32;
    let scale = width.max(height) / 2.0;

    Vec2F32::new(x * scale + width / 2.0, y * scale + height / 2.0)
}

fn check_camera_info(info: &CameraInfo, name: &str) -> Result<()> {
    ensure!(
        info.width > 0 && info.height > 0,
        "{name} camera size is zero"
    );
    ensure!(
        info.p[0] != 0.0 && info.p[5] != 0.0,
        "{name} camera focal length is zero"
    );
    ensure!(
        info.p[10] != 0.0,
        "{name} camera projection matrix is uninitialized"
    );

    Ok(())
}

fn ensure_same_intrinsics(left: &CameraInfo, right: &CameraInfo) -> Result<()> {
    for (left_value, right_value) in [
        (left.p[0], right.p[0]),
        (left.p[5], right.p[5]),
        (left.p[2], right.p[2]),
        (left.p[6], right.p[6]),
    ] {
        if (left_value - right_value).abs() > CAMERA_EPSILON {
            bail!("left and right rectified intrinsics differ");
        }
    }

    Ok(())
}

fn projection_x(info: &CameraInfo) -> f64 {
    info.p[3] / info.p[0]
}
