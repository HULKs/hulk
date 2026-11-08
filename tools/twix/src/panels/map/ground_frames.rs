//! Brings ground data of different times into one picture.
//!
//! Ground moves with the robot, and messages arrive at their own rates and after their own delays,
//! so the latest samples of two topics generally do not refer to the same ground frame. Odometry
//! relates the ground frames of all times, and each localization estimate holds together with the
//! odometry pose of its own time. Together they place any ground frame in the field, as long as the
//! poses are known; without them, data is left out rather than drawn at some other place.

use std::time::Duration;

use coordinate_systems::{Field, Ground, Odometry};
use linear_algebra::{Isometry2, Pose2};
use ros_z::time::Time;
use ros_z_debug::{RetentionPolicy, TopicObservation};

use crate::backend::RobotBackend;

/// How long poses and localization estimates are kept for data arriving late.
const RETENTION: Duration = Duration::from_secs(2);
/// Poses are 2 ms apart; beyond this distance, the robot could have moved unnoticed.
const MAXIMUM_POSE_DISTANCE: Duration = Duration::from_millis(10);
/// How far a localization estimate may lie from the time of data without any odometry to bridge.
const MAXIMUM_LOCALIZATION_DISTANCE: Duration = Duration::from_millis(100);

pub struct GroundFrames {
    odometry: TopicObservation<Pose2<Odometry>>,
    ground_to_field: TopicObservation<Isometry2<Ground, Field>>,
}

impl GroundFrames {
    pub fn new(backend: &RobotBackend) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();
        let retention =
            RetentionPolicy::time_window(RETENTION).expect("retention must be non-zero");
        Self {
            odometry: backend
                .observer()
                .observe_typed("inputs/odometry")
                .expect("failed to construct odometry observer")
                .retention(retention)
                .spawn(),
            ground_to_field: backend
                .observer()
                .observe_typed("ground_to_field")
                .expect("failed to construct ground_to_field observer")
                .retention(retention)
                .spawn(),
        }
    }

    /// The relation of all ground frames to the field and to the current one, as known right now.
    pub fn alignment(&self) -> GroundAlignment<'_> {
        GroundAlignment {
            frames: self,
            odometry_to_field: self.odometry_to_field(),
            current_pose: self.odometry.latest().map(|record| record.value),
        }
    }

    fn pose_at(&self, time: Time) -> Option<Pose2<Odometry>> {
        let record = self.odometry.get_nearest(time)?;
        (record.source_time.abs_diff(time) <= MAXIMUM_POSE_DISTANCE).then_some(record.value)
    }

    /// The newest localization estimate whose odometry pose is known, relative to the odometry.
    fn odometry_to_field(&self) -> Option<Isometry2<Odometry, Field>> {
        let anchor = |time, ground_to_field: Isometry2<Ground, Field>| {
            let pose = self.pose_at(time)?;
            Some(ground_to_field * pose.as_transform::<Ground>().inverse())
        };
        let latest = self.ground_to_field.latest()?;
        anchor(latest.source_time, latest.value).or_else(|| {
            self.ground_to_field
                .get_all()
                .iter()
                .rev()
                .find_map(|record| anchor(record.source_time, record.value))
        })
    }
}

pub struct GroundAlignment<'a> {
    frames: &'a GroundFrames,
    odometry_to_field: Option<Isometry2<Odometry, Field>>,
    current_pose: Option<Pose2<Odometry>>,
}

impl GroundAlignment<'_> {
    /// Where the robot's current ground frame is in the field.
    pub fn current_ground_to_field(&self) -> Option<Isometry2<Ground, Field>> {
        match self.current_pose {
            Some(pose) => Some(self.odometry_to_field? * pose.as_transform::<Ground>()),
            None => self
                .frames
                .ground_to_field
                .latest()
                .map(|record| record.value),
        }
    }

    /// Where the ground frame of `time` is in the field, by the latest localization.
    pub fn ground_to_field_at(&self, time: Time) -> Option<Isometry2<Ground, Field>> {
        if self.current_pose.is_none() {
            let record = self.frames.ground_to_field.get_nearest(time)?;
            return (record.source_time.abs_diff(time) <= MAXIMUM_LOCALIZATION_DISTANCE)
                .then_some(record.value);
        }
        Some(self.odometry_to_field? * self.frames.pose_at(time)?.as_transform::<Ground>())
    }

    /// Where the ground frame of `time` is in the current ground frame.
    ///
    /// Without any odometry, the current ground frame is unknown and the data is taken as current.
    pub fn ground_to_current_ground(&self, time: Time) -> Option<Isometry2<Ground, Ground>> {
        let Some(current_pose) = self.current_pose else {
            return Some(Isometry2::identity());
        };
        let pose = self.frames.pose_at(time)?;
        Some(current_pose.as_transform::<Ground>().inverse() * pose.as_transform::<Ground>())
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::sync::Arc;

    use eframe::{
        egui::{Context, RawInput, Rect, pos2, vec2},
        epaint::{Color32, Pos2, Shape},
    };
    use linear_algebra::{point, vector};
    use ros_z::pubsub::Publisher;
    use tokio::runtime::Handle;
    use twix_visualization::twix_painter::{Orientation, TwixPainter};
    use types::field_dimensions::FieldDimensions;

    use super::*;
    use crate::panels::map::{layer::Layer, layers::BallPosition};

    pub(in crate::panels::map) fn time(milliseconds: i64) -> Time {
        Time::from_nanos((1_000 + milliseconds) * 1_000_000)
    }

    pub(in crate::panels::map) struct Fixture {
        pub backend: Arc<RobotBackend>,
        pub namespace: String,
        odometry: Publisher<Pose2<Odometry>>,
        ground_to_field: Publisher<Isometry2<Ground, Field>>,
    }

    impl Fixture {
        pub async fn new() -> Self {
            let namespace = format!("/map_frames_{}", uuid::Uuid::new_v4().simple());
            let backend = Arc::new(
                RobotBackend::new(Handle::current(), None, namespace.clone())
                    .await
                    .unwrap(),
            );
            // Main's query backend windows history around its cursor. Keep the fixture clock
            // in the same time domain as the synthetic publications.
            backend
                .query_connection()
                .cursor
                .set(ros_z_debug::query::Cursor::At(time(1_000)));
            let node = backend.node();
            Self {
                odometry: node
                    .publisher(&format!("{namespace}/inputs/odometry"))
                    .build()
                    .await
                    .unwrap(),
                ground_to_field: node
                    .publisher(&format!("{namespace}/ground_to_field"))
                    .build()
                    .await
                    .unwrap(),
                backend,
                namespace,
            }
        }

        pub async fn publisher<T: ros_z::Message>(&self, topic: &str) -> Publisher<T> {
            self.backend
                .node()
                .publisher(&format!("{}/{topic}", self.namespace))
                .build()
                .await
                .unwrap()
        }

        pub async fn publish_pose(&self, milliseconds: i64, pose: Pose2<Odometry>) {
            self.odometry
                .publish_with_source_time(&pose, time(milliseconds))
                .await
                .unwrap();
        }

        pub async fn publish_localization(
            &self,
            milliseconds: i64,
            ground_to_field: Isometry2<Ground, Field>,
        ) {
            self.ground_to_field
                .publish_with_source_time(&ground_to_field, time(milliseconds))
                .await
                .unwrap();
        }
    }

    /// Repeats `publish` until `condition`, since observers connect in the background.
    pub(in crate::panels::map) async fn eventually<P, F>(
        mut publish: impl FnMut() -> P,
        condition: F,
    ) where
        P: Future<Output = ()>,
        F: Fn() -> bool,
    {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                publish().await;
                if condition() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("observers did not receive the data");
    }

    const WORLD_BALL: [f32; 2] = [3.0, 0.0];

    fn pose_after_turn() -> Pose2<Odometry> {
        Pose2::new(point![0.0, 0.0], 0.02)
    }

    fn ground_to_field_of(pose: Pose2<Odometry>) -> Isometry2<Ground, Field> {
        Isometry2::wrap(pose.as_transform::<Ground>().inner)
    }

    fn render_in_field(paint: impl Fn(&TwixPainter<Field>)) -> (Vec<Pos2>, Pos2, f32) {
        let context = Context::default();
        let mut expected = pos2(0.0, 0.0);
        let mut scale = 1.0;
        let output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 800.0))),
                ..Default::default()
            },
            |ui| {
                let (_, painter) = TwixPainter::<Field>::allocate(
                    ui,
                    vector![10.0, 10.0],
                    point![5.0, -5.0],
                    Orientation::RightHanded,
                );
                expected = painter.transform_world_to_pixel(point![WORLD_BALL[0], WORLD_BALL[1]]);
                scale = painter.scaling();
                paint(&painter);
            },
        );
        let centers = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                Shape::Circle(circle) if circle.fill == Color32::WHITE => Some(circle.center),
                _ => None,
            })
            .collect();
        (centers, expected, scale)
    }

    fn resting_ball() -> Option<types::ball_position::BallPosition<Ground>> {
        Some(types::ball_position::BallPosition {
            position: point![WORLD_BALL[0], WORLD_BALL[1]],
            velocity: vector![0.0, 0.0],
            last_seen: time(0),
        })
    }

    /// The ball was seen before the turn, so the newer poses and estimates must not turn it along.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn resting_ball_stays_when_only_newer_frames_arrive() {
        let fixture = Fixture::new().await;
        let balls = fixture
            .publisher::<Option<types::ball_position::BallPosition<Ground>>>(
                "ball_filter/ball_position",
            )
            .await;
        let layer = BallPosition::new(fixture.backend.clone());
        let alignment_ready = || {
            layer
                .ground_frames()
                .alignment()
                .ground_to_field_at(time(20))
                .is_some()
                && layer.ball_is_known()
        };
        eventually(
            || async {
                fixture.publish_pose(0, Pose2::zero()).await;
                fixture.publish_localization(0, Isometry2::identity()).await;
                balls
                    .publish_with_source_time(&resting_ball(), time(0))
                    .await
                    .unwrap();
                fixture.publish_pose(20, pose_after_turn()).await;
                fixture
                    .publish_localization(20, ground_to_field_of(pose_after_turn()))
                    .await;
            },
            alignment_ready,
        )
        .await;

        let (centers, expected, scale) = render_in_field(|painter| {
            layer.paint(painter, &FieldDimensions::SPL_2025).unwrap();
        });

        let error = centers[0].distance(expected) / scale;
        assert!(error < 1e-3, "the ball moved by {error} m");
    }

    /// Without odometry only a localization estimate of about the ball's time can place it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn resting_ball_falls_back_to_the_estimate_of_its_time() {
        let fixture = Fixture::new().await;
        let balls = fixture
            .publisher::<Option<types::ball_position::BallPosition<Ground>>>(
                "ball_filter/ball_position",
            )
            .await;
        let layer = BallPosition::new(fixture.backend.clone());
        eventually(
            || async {
                fixture.publish_localization(0, Isometry2::identity()).await;
                balls
                    .publish_with_source_time(&resting_ball(), time(0))
                    .await
                    .unwrap();
                fixture
                    .publish_localization(20, Isometry2::from_parts(vector![0.0, 0.0], 0.02))
                    .await;
            },
            || {
                layer
                    .ground_frames()
                    .ground_to_field
                    .latest()
                    .is_some_and(|record| record.source_time == time(20))
                    && layer.ball_is_known()
            },
        )
        .await;

        let (centers, expected, scale) = render_in_field(|painter| {
            layer.paint(painter, &FieldDimensions::SPL_2025).unwrap();
        });

        let error = centers[0].distance(expected) / scale;
        assert!(error < 1e-3, "the ball moved by {error} m");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn estimates_and_data_without_a_pose_of_their_time_are_not_used() {
        let fixture = Fixture::new().await;
        let frames = GroundFrames::new(&fixture.backend);
        let reference = Isometry2::<Ground, Field>::from_parts(vector![1.0, 2.0], 0.0);
        let pose = Pose2::new(point![0.5, 0.0], 0.0);
        eventually(
            || async {
                fixture.publish_pose(0, Pose2::zero()).await;
                fixture.publish_localization(0, reference).await;
                // Odometry stops; this estimate cannot be related to any pose.
                fixture
                    .publish_localization(500, Isometry2::from_parts(vector![9.0, 9.0], 1.0))
                    .await;
                fixture.publish_pose(50, pose).await;
            },
            || {
                frames
                    .ground_to_field
                    .get_nearest(time(500))
                    .is_some_and(|record| record.source_time == time(500))
                    && frames.pose_at(time(50)).is_some()
                    && frames.pose_at(time(0)).is_some()
            },
        )
        .await;

        let alignment = frames.alignment();

        // The newest estimate that holds together with a pose places the current ground frame.
        let current = alignment.current_ground_to_field().unwrap().as_pose();
        assert_eq!(current.position(), point![1.5, 2.0]);
        assert_eq!(
            alignment
                .ground_to_field_at(time(0))
                .unwrap()
                .as_pose()
                .position(),
            point![1.0, 2.0]
        );
        assert_eq!(
            alignment
                .ground_to_current_ground(time(0))
                .unwrap()
                .as_pose()
                .position(),
            point![-0.5, 0.0]
        );
        // There is no pose close to this time, so nothing is known about its frame.
        assert!(alignment.ground_to_field_at(time(25)).is_none());
        assert!(alignment.ground_to_current_ground(time(25)).is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn without_odometry_ground_data_is_taken_as_current() {
        let fixture = Fixture::new().await;
        let frames = GroundFrames::new(&fixture.backend);
        let alignment = frames.alignment();

        assert_eq!(
            alignment.ground_to_current_ground(time(0)),
            Some(Isometry2::identity())
        );
        assert!(alignment.current_ground_to_field().is_none());
        assert!(alignment.ground_to_field_at(time(0)).is_none());
    }
}
