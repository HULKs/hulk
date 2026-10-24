use std::{
    f32::consts::{FRAC_PI_2, PI, TAU},
    sync::Arc,
    time::Duration,
};

use color_eyre::Report;
use coordinate_systems::{Camera, Field, Pixel};
use eframe::egui::{Color32, Stroke};
use linear_algebra::{Isometry3, Point2, Point3, point};
use projection::{camera_matrix::CameraMatrix, intrinsic::Intrinsic};
use ros_z::{
    qos::{QosDurability, QosProfile},
    time::Time,
};
use ros_z_debug::{
    ObservationPolicy, SampleRecord,
    query::{TemporalMatch, TimeProjection, plan::Selection},
};
use types::{
    camera_geometry::{CameraGeometry, interpolate_camera_geometry},
    field_dimensions::{FieldDimensions, Half, Side},
    localization::{LocalizationEstimate, LocalizationStatus},
    time_wrapper::TimeWrapper,
    visual_localization::VisualLocalizationFrame,
};

use super::super::{
    image_overlay::ImageOverlay,
    inputs::{OverlayInput, OverlayInputs},
};
use twix_visualization::twix_painter::TwixPainter;

const NEAR_Z: f32 = 1.0e-4;
const FIELD_STROKE: Stroke = Stroke {
    width: 2.0,
    color: Color32::from_rgb(80, 220, 255),
};
const RESIDUAL_STROKE: Stroke = Stroke {
    width: 2.0,
    color: Color32::from_rgb(255, 80, 200),
};

#[derive(Default)]
pub(in crate::panels::image) struct ProjectedFieldLinesOverlay {
    sample: Option<ProjectedFieldSample>,
}

impl ImageOverlay for ProjectedFieldLinesOverlay {
    fn inputs(&self) -> Vec<OverlayInput> {
        let bracket = Selection::Bracket {
            max_gap: Duration::from_secs(2),
        };
        let mut dimensions =
            OverlayInput::new::<FieldDimensions>("field_dimensions", Selection::latest());
        dimensions.source.projection = TimeProjection::SourceTime;
        let mut status =
            OverlayInput::new::<LocalizationStatus>("localization/status", Selection::latest());
        let latched = ObservationPolicy::default().with_subscriber_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        });
        dimensions.source.live_policy = latched;
        status.source.live_policy = latched;
        vec![
            OverlayInput::new::<TimeWrapper<CameraMatrix>>("camera_matrix", bracket.clone()),
            OverlayInput::new::<LocalizationEstimate>("localization/estimate", bracket),
            status,
            dimensions,
            OverlayInput::new::<TimeWrapper<VisualLocalizationFrame>>(
                "field_mark_association/visual_localization_local",
                Selection::Exact,
            ),
        ]
    }

    fn prepare(&mut self, image_time: Time, inputs: &mut OverlayInputs) -> Result<(), Report> {
        self.sample = None;
        let camera = inputs.decode::<TimeWrapper<CameraMatrix>>(0, image_time)?;
        let poses = inputs.decode::<LocalizationEstimate>(1, image_time)?;
        let status = inputs.decode::<LocalizationStatus>(2, image_time)?;
        let dimensions = inputs
            .decode::<FieldDimensions>(3, image_time)
            .ok()
            .flatten();
        let associations = inputs
            .decode::<TimeWrapper<VisualLocalizationFrame>>(4, image_time)
            .ok()
            .flatten();
        self.sample = (|| {
            let status = status.as_ref()?.last();
            let epoch = status.value.epoch;
            let generation = status.value.generation;
            let (before, after) = match poses.as_ref()? {
                TemporalMatch::Exact(record) => (record, record),
                TemporalMatch::Bracket { before, after } => (before, after),
                TemporalMatch::Before(_) => return None,
            };
            if [before, after]
                .iter()
                .any(|p| p.value.epoch != epoch || p.value.generation != generation)
            {
                return None;
            }
            let gap = after.value.time.duration_since(before.value.time);
            let fraction = if gap.is_zero() {
                0.0
            } else {
                image_time.duration_since(before.value.time).as_secs_f32() / gap.as_secs_f32()
            };
            // Rendering interpolates its own snapshots; the localization node publishes only solves.
            let field_to_robot = super::camera::interpolate_transform(
                Isometry3::wrap(
                    before
                        .value
                        .robot_to_field?
                        .pose
                        .inner
                        .cast::<f32>()
                        .inverse(),
                ),
                Isometry3::wrap(
                    after
                        .value
                        .robot_to_field?
                        .pose
                        .inner
                        .cast::<f32>()
                        .inverse(),
                ),
                fraction,
            );
            let (before, after) = match camera.as_ref()? {
                TemporalMatch::Exact(record) => (record, record),
                TemporalMatch::Bracket { before, after } => (before, after),
                TemporalMatch::Before(_) => return None,
            };
            let geometry = |record: &Arc<SampleRecord<TimeWrapper<CameraMatrix>>>| TimeWrapper {
                time: record.value.time,
                inner: CameraGeometry::from(&record.value.inner),
            };
            let camera = interpolate_camera_geometry(
                &geometry(before),
                &geometry(after),
                image_time,
                Duration::from_secs(2),
            )?;
            let sample = ProjectedFieldSample {
                projection: FieldProjection {
                    field_to_camera: camera.robot_to_camera * field_to_robot,
                    intrinsics: camera.intrinsics,
                },
                dimensions: dimensions
                    .as_ref()
                    .map_or(FieldDimensions::SPL_2025, |p| p.last().value),
                associations: associations.as_ref().map(|p| p.last().clone()).filter(|p| {
                    p.value.inner.epoch == epoch && p.value.inner.generation == generation
                }),
            };
            sample
                .projection
                .field_to_camera
                .inner
                .to_homogeneous()
                .iter()
                .all(|v| v.is_finite())
                .then_some(sample)
        })();
        Ok(())
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) {
        let Some(sample) = &self.sample else {
            return;
        };
        sample.projection.draw_field(painter, &sample.dimensions);
        if let Some(frame) = &sample.associations {
            for association in &frame.value.inner.associations {
                if !association.detection.inner.iter().all(|v| v.is_finite()) {
                    continue;
                }
                if let Some(projected) = sample.projection.project(association.field_point) {
                    painter.line_segment(association.detection, projected, RESIDUAL_STROKE);
                    painter.circle_filled(projected, 3.5, RESIDUAL_STROKE.color);
                }
            }
        }
    }
}

pub(in crate::panels::image) struct ProjectedFieldSample {
    projection: FieldProjection,
    dimensions: FieldDimensions,
    associations: Option<Arc<SampleRecord<TimeWrapper<VisualLocalizationFrame>>>>,
}

struct FieldProjection {
    field_to_camera: Isometry3<Field, Camera>,
    intrinsics: Intrinsic,
}

impl FieldProjection {
    fn project(&self, point: Point3<Field>) -> Option<Point2<Pixel>> {
        self.project_camera(self.field_to_camera * point)
    }

    fn project_camera(&self, point: Point3<Camera>) -> Option<Point2<Pixel>> {
        if !point.inner.iter().all(|v| v.is_finite())
            || point.z() < NEAR_Z
            || !self.intrinsics.is_valid()
        {
            return None;
        }
        let pixel = self.intrinsics.project(point.coords());
        pixel.inner.iter().all(|v| v.is_finite()).then_some(pixel)
    }

    fn segment(&self, start: Point2<Field>, end: Point2<Field>) -> Option<[Point2<Pixel>; 2]> {
        let mut start = self.field_to_camera * start.extend(0.0);
        let mut end = self.field_to_camera * end.extend(0.0);
        if !start
            .inner
            .iter()
            .chain(end.inner.iter())
            .all(|v| v.is_finite())
            || (start.z() < NEAR_Z && end.z() < NEAR_Z)
        {
            return None;
        }
        if start.z() < NEAR_Z || end.z() < NEAR_Z {
            let t = (NEAR_Z - start.z()) / (end.z() - start.z());
            let mut clipped = start + (end - start) * t;
            // Roundoff in interpolation must not put the clipped endpoint behind the plane.
            clipped.inner.z = NEAR_Z;
            if start.z() < NEAR_Z {
                start = clipped;
            } else {
                end = clipped;
            }
        }
        Some([self.project_camera(start)?, self.project_camera(end)?])
    }

    fn draw_field(&self, painter: &TwixPainter<Pixel>, d: &FieldDimensions) {
        let line = |a, b| {
            if let Some([a, b]) = self.segment(a, b) {
                painter.line_segment(a, b, FIELD_STROKE);
            }
        };
        line(d.t_crossing(Side::Left), d.t_crossing(Side::Right));
        for side in [Side::Left, Side::Right] {
            line(d.corner(Half::Own, side), d.corner(Half::Opponent, side));
        }
        self.arc(
            painter,
            d.center(),
            d.center_circle_diameter / 2.0,
            0.0,
            TAU,
        );
        for half in [Half::Own, Half::Opponent] {
            line(d.corner(half, Side::Left), d.corner(half, Side::Right));
            line(
                d.goal_box_corner(half, Side::Left),
                d.goal_box_corner(half, Side::Right),
            );
            line(
                d.penalty_box_corner(half, Side::Left),
                d.penalty_box_corner(half, Side::Right),
            );
            let spot = d.penalty_spot(half);
            let r = d.penalty_marker_size / 2.0;
            line(point![spot.x() - r, 0.0], point![spot.x() + r, 0.0]);
            line(point![spot.x(), -r], point![spot.x(), r]);
            for side in [Side::Left, Side::Right] {
                line(
                    d.goal_box_corner(half, side),
                    d.goal_box_goal_line_intersection(half, side),
                );
                line(
                    d.penalty_box_corner(half, side),
                    d.penalty_box_goal_line_intersection(half, side),
                );
                let start = match (half, side) {
                    (Half::Own, Side::Left) => -FRAC_PI_2,
                    (Half::Own, Side::Right) => 0.0,
                    (Half::Opponent, Side::Left) => PI,
                    (Half::Opponent, Side::Right) => FRAC_PI_2,
                };
                self.arc(
                    painter,
                    d.corner(half, side),
                    d.corner_arc_radius,
                    start,
                    start + FRAC_PI_2,
                );
            }
        }
    }

    fn arc(
        &self,
        painter: &TwixPainter<Pixel>,
        center: Point2<Field>,
        radius: f32,
        start: f32,
        end: f32,
    ) {
        if !radius.is_finite() || radius <= 0.0 {
            return;
        }
        let count = ((end - start).abs() / TAU * 256.0).ceil().clamp(1.0, 256.0) as usize;
        let at = |angle: f32| {
            self.project(point![
                center.x() + radius * angle.cos(),
                center.y() + radius * angle.sin(),
                0.0
            ])
        };
        let mut previous = at(start);
        for index in 1..=count {
            let next = at(start + (end - start) * index as f32 / count as f32);
            if let (Some(a), Some(b)) = (previous, next) {
                painter.line_segment(a, b, FIELD_STROKE);
            }
            previous = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panels::image::tests::raw_sample;
    use ros_z_debug::query::plan::State;
    use types::localization::{LocalizationState, PoseEstimate};

    #[test]
    fn exposure_snapshot_requires_matching_pose_generation_and_recovers_delayed_inputs() {
        let camera = |time| {
            raw_sample(
                "/camera_matrix",
                time as u64,
                time,
                time,
                &TimeWrapper {
                    time: Time::from_nanos(time),
                    inner: CameraMatrix::default(),
                },
            )
        };
        let pose = |time, generation, tracked: bool| {
            raw_sample(
                "/localization/estimate",
                time as u64,
                time,
                time,
                &LocalizationEstimate {
                    time: Time::from_nanos(time),
                    epoch: 1,
                    generation,
                    robot_to_local: PoseEstimate {
                        pose: Isometry3::identity(),
                        covariance: nalgebra::SMatrix::zeros(),
                    },
                    robot_to_field: tracked.then_some(PoseEstimate {
                        pose: Isometry3::identity(),
                        covariance: nalgebra::SMatrix::zeros(),
                    }),
                },
            )
        };
        let status = raw_sample(
            "/localization/status",
            1,
            0,
            0,
            &LocalizationStatus {
                time: Time::from_nanos(0),
                epoch: 1,
                generation: 2,
                state: LocalizationState::Tracking,
                heading: None,
            },
        );
        let mut states = vec![
            State::Ready(vec![camera(10), camera(30)]),
            State::Pending,
            State::Ready(vec![status]),
            State::Ready(vec![]),
            State::Ready(vec![]),
        ];
        let mut inputs = OverlayInputs::default();
        let mut overlay = ProjectedFieldLinesOverlay::default();
        let time = Time::from_nanos(20);
        inputs.update(&states);
        overlay.prepare(time, &mut inputs).unwrap();
        assert!(overlay.sample.is_none());
        states[1] = State::Ready(vec![pose(10, 2, true), pose(30, 2, true)]);
        inputs.update(&states);
        overlay.prepare(time, &mut inputs).unwrap();
        assert!(overlay.sample.is_some());
        for (generation, tracked) in [(3, true), (2, false)] {
            states[1] = State::Ready(vec![pose(10, 2, true), pose(30, generation, tracked)]);
            inputs.update(&states);
            overlay.prepare(time, &mut inputs).unwrap();
            assert!(overlay.sample.is_none());
        }
    }

    #[test]
    fn projection_rejects_invalid_depth_and_clips_crossing_segments() {
        let projection = FieldProjection {
            field_to_camera: Isometry3::from_rotation(linear_algebra::vector![
                0.0, -FRAC_PI_2, 0.0
            ]),
            intrinsics: Intrinsic::default(),
        };
        assert!(projection.project(point![f32::NAN, 0.0, 0.0]).is_none());
        assert!(
            projection
                .segment(point![-2.0, 1.0], point![-1.0, 1.0])
                .is_none()
        );
        let segment = projection
            .segment(point![-1.0, 1.0], point![1.0, 1.0])
            .unwrap();
        assert!(
            segment
                .iter()
                .all(|p| p.inner.iter().all(|v| v.is_finite()))
        );
        assert_eq!(
            projection
                .segment(point![1.0, 1.0], point![-1.0, 1.0])
                .unwrap(),
            [segment[1], segment[0]]
        );
    }
}
