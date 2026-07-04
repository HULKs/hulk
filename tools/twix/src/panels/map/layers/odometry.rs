use std::{num::NonZeroUsize, sync::Arc, time::Duration};

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::{Field, Odometry as OdometryFrame};
use linear_algebra::{Point2, Pose2, point};
use ros_z_debug::{RetentionPolicy, SampleRecord, TopicObservation};
use types::field_dimensions::FieldDimensions;

use crate::{backend::RobotBackend, panels::map::layer::Layer, twix_painter::TwixPainter};

pub struct Odometry {
    pose: TopicObservation<Pose2<OdometryFrame>>,
    booster_pose: TopicObservation<Pose2<OdometryFrame>>,
}

const TRAIL_DURATION: Duration = Duration::from_secs(60 * 60);
const TRAIL_MAX_SAMPLES: usize = 100_000;

impl Layer<Field> for Odometry {
    const NAME: &'static str = "Odometry";

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let retention = RetentionPolicy::time_window_with_max_samples(
            TRAIL_DURATION,
            NonZeroUsize::new(TRAIL_MAX_SAMPLES).expect("trail sample cap must be non-zero"),
        )
        .expect("trail retention duration must be non-zero");

        let pose = backend
            .observer()
            .observe_typed("inputs/odometry")
            .expect("failed to construct odometry observer")
            .retention(retention)
            .spawn();

        let booster_pose = backend
            .observer()
            .observe_typed("inputs/booster_odometry")
            .expect("failed to construct Booster odometry observer")
            .retention(retention)
            .spawn();

        Self { pose, booster_pose }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        paint_odometry_source(painter, &self.pose, Color32::CYAN);
        paint_odometry_source(painter, &self.booster_pose, Color32::RED);

        Ok(())
    }
}

fn paint_odometry_source(
    painter: &TwixPainter<Field>,
    observation: &TopicObservation<Pose2<OdometryFrame>>,
    color: Color32,
) {
    let trail_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 96);
    painter.polyline(
        observation
            .get_all()
            .iter()
            .map(|sample| odometry_position_to_field(sample.value)),
        Stroke {
            width: 0.01,
            color: trail_color,
        },
    );

    if let Some(SampleRecord { value: pose, .. }) = observation.latest().as_deref() {
        let pose = odometry_pose_to_field(*pose);
        let stroke = Stroke {
            width: 0.02,
            color: Color32::BLACK,
        };
        painter.pose(pose, 0.15, 0.25, color, stroke);
    }
}

fn odometry_pose_to_field(pose: Pose2<OdometryFrame>) -> Pose2<Field> {
    Pose2::<Field>::new(odometry_position_to_field(pose), pose.orientation().angle())
}

fn odometry_position_to_field(pose: Pose2<OdometryFrame>) -> Point2<Field> {
    point![<Field>, pose.position().x(), pose.position().y()]
}
