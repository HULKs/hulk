use std::{
    f32::consts::{FRAC_PI_2, PI, TAU},
    sync::Arc,
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
use ros_z_debug::{ObservationPolicy, SampleRecord};
use types::{
    field_dimensions::{FieldDimensions, Half, Side},
    localization::{LocalizationEstimate, LocalizationStatus},
    time_wrapper::TimeWrapper,
    visual_localization::VisualLocalizationFrame,
};

use super::super::image_overlay::{
    ImageOverlay, OverlayObservation, interpolate_transform, valid_intrinsics,
};
use crate::repaint::ObservationContext;
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

pub(in crate::panels::image) struct ProjectedFieldLinesOverlay {
    camera_matrix: OverlayObservation<TimeWrapper<CameraMatrix>>,
    localization: OverlayObservation<LocalizationEstimate>,
    status: OverlayObservation<LocalizationStatus>,
    dimensions: OverlayObservation<FieldDimensions>,
    associations: OverlayObservation<TimeWrapper<VisualLocalizationFrame>>,
}

impl ImageOverlay for ProjectedFieldLinesOverlay {
    type Sample = ProjectedFieldSample;
    const NAME: &'static str = "Projected Field Lines";
    const STORAGE_KEY: &'static str = "projected_field_lines";

    fn new<C: ObservationContext>(context: &C) -> Result<Self, Report> {
        let latched = ObservationPolicy::default().with_subscriber_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        });
        Ok(Self {
            camera_matrix: OverlayObservation::new(context, "camera_matrix")?,
            localization: OverlayObservation::new(context, "localization/estimate")?,
            status: OverlayObservation::with_policy(context, "localization/status", latched)?,
            dimensions: OverlayObservation::with_policy(context, "field_dimensions", latched)?,
            associations: OverlayObservation::new(
                context,
                "field_mark_association/visual_localization_local",
            )?,
        })
    }

    fn prepare(&self, image_time: Time) -> Option<Self::Sample> {
        let status = self.status.latest()?;
        let epoch = status.value.epoch;
        let generation = status.value.generation;
        let poses = self.history();
        let before = poses
            .iter()
            .rev()
            .filter(|p| {
                p.value.time <= image_time
                    && p.value.epoch == epoch
                    && p.value.generation == generation
            })
            .max_by_key(|p| p.value.time)?;
        let after = if before.value.time == image_time {
            before
        } else {
            poses
                .iter()
                .rev()
                .filter(|p| {
                    p.value.time >= image_time
                        && p.value.epoch == epoch
                        && p.value.generation == generation
                })
                .min_by_key(|p| p.value.time)?
        };
        let gap = after.value.time.duration_since(before.value.time);
        let fraction = if gap.is_zero() {
            0.0
        } else {
            image_time.duration_since(before.value.time).as_secs_f32() / gap.as_secs_f32()
        };
        // Rendering interpolates its own snapshots; the localization node publishes only solves.
        let field_to_robot = interpolate_transform(
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
        )?;
        let camera = self.camera_matrix.camera_at(image_time)?;
        let sample = ProjectedFieldSample {
            projection: FieldProjection {
                field_to_camera: camera.robot_to_camera * field_to_robot,
                intrinsics: camera.intrinsics,
            },
            dimensions: self
                .dimensions
                .latest()
                .map_or(FieldDimensions::SPL_2025, |p| p.value),
            epoch,
            generation,
            pose_time: before.value.time,
            associations: self
                .associations
                .at_time(image_time)
                .filter(|p| p.value.inner.epoch == epoch && p.value.inner.generation == generation),
        };
        self.valid(&sample).then_some(sample)
    }

    fn paint(painter: &TwixPainter<Pixel>, sample: &Self::Sample) {
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

impl ProjectedFieldLinesOverlay {
    #[cfg(test)]
    pub(in crate::panels::image) fn for_test(
        context: &crate::panel::PanelCreationContext<'_>,
        node: Arc<ros_z::node::Node>,
    ) -> Self {
        use super::super::image_overlay::tests::observation;
        Self {
            camera_matrix: observation(context, Arc::clone(&node), "camera_matrix"),
            localization: observation(context, Arc::clone(&node), "localization/estimate"),
            status: observation(context, Arc::clone(&node), "localization/status"),
            dimensions: observation(context, Arc::clone(&node), "field_dimensions"),
            associations: observation(
                context,
                node,
                "field_mark_association/visual_localization_local",
            ),
        }
    }

    pub(in crate::panels::image) fn unavailable(&self, time: Time) -> bool {
        let Some(status) = self.status.latest() else {
            return false;
        };
        self.history()
            .iter()
            .rev()
            .filter(|p| p.value.time <= time)
            .max_by_key(|p| p.value.time)
            .is_some_and(|p| {
                p.value.epoch != status.value.epoch
                    || p.value.generation != status.value.generation
                    || p.value.robot_to_field.is_none()
            })
    }

    pub(in crate::panels::image) fn enrich_residual(
        &self,
        sample: &mut ProjectedFieldSample,
        time: Time,
    ) {
        if sample.associations.is_none() {
            sample.associations = self.associations.at_time(time).filter(|p| {
                p.value.inner.epoch == sample.epoch && p.value.inner.generation == sample.generation
            });
        }
    }

    pub(in crate::panels::image) fn valid(&self, sample: &ProjectedFieldSample) -> bool {
        self.status.latest().is_some_and(|status| {
            status.value.epoch == sample.epoch && status.value.generation == sample.generation
        }) && !self.history().iter().any(|p| {
            p.value.time >= sample.pose_time
                && p.value.epoch == sample.epoch
                && p.value.generation == sample.generation
                && p.value.robot_to_field.is_none()
        })
    }

    fn history(&self) -> Vec<Arc<SampleRecord<LocalizationEstimate>>> {
        let mut samples = self.localization.get_all();
        // A late measurement can produce a newer solution at the same pose time.
        samples.sort_by_key(|sample| {
            (
                sample.value.epoch,
                sample.value.generation,
                sample.value.time,
            )
        });
        samples.reverse();
        samples.dedup_by_key(|sample| {
            (
                sample.value.epoch,
                sample.value.generation,
                sample.value.time,
            )
        });
        samples.reverse();
        samples
    }
}

pub(in crate::panels::image) struct ProjectedFieldSample {
    projection: FieldProjection,
    dimensions: FieldDimensions,
    epoch: u64,
    generation: u64,
    pose_time: Time,
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
            || !valid_intrinsics(&self.intrinsics)
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
