use std::{boxed::Box, future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use coordinate_systems::{Field, Ground, Robot};
use linear_algebra::{Isometry2, Isometry3};
use ros_z::{cache::Cache, context::Context, time::Time};
use types::{time_wrapper::TimeWrapper, visual_localization::LOCALIZATION_POSE_3D_TOPIC};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

const MAX_ROBOT_TO_GROUND_TIME_DISTANCE: Duration = Duration::from_millis(100);

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("localization2d").build().await?;

    let localization_subscriber = node
        .subscriber::<TimeWrapper<Option<Isometry3<Field, Robot>>>>(LOCALIZATION_POSE_3D_TOPIC)
        .build()
        .await?;
    let robot_to_ground_cache = node
        .subscriber::<TimeWrapper<Option<Isometry3<Robot, Ground>>>>("robot_to_ground")
        .cache(128)
        .with_stamp(|wrapper| wrapper.time)
        .build()
        .await?;

    let ground_to_field_publisher = node
        .publisher::<Isometry2<Ground, Field>>("ground_to_field")
        .build()
        .await?;

    loop {
        let localization = localization_subscriber.recv().await?;
        let time = localization.time;
        let Some(field_to_robot) = localization.inner else {
            continue;
        };

        let Some(robot_to_ground) = fresh_robot_to_ground_at(&robot_to_ground_cache, time) else {
            continue;
        };

        ground_to_field_publisher
            .publish_with_source_time(
                &ground_to_field_from_field_to_robot(field_to_robot, robot_to_ground),
                time,
            )
            .await?;
    }
}

fn fresh_robot_to_ground_at(
    robot_to_ground_cache: &Cache<TimeWrapper<Option<Isometry3<Robot, Ground>>>>,
    time: Time,
) -> Option<Isometry3<Robot, Ground>> {
    robot_to_ground_cache
        .get_nearest_with_stamp(time)
        .filter(|(stamp, _)| time_distance(*stamp, time) <= MAX_ROBOT_TO_GROUND_TIME_DISTANCE)
        .and_then(|(_, transform)| transform.inner)
}

fn time_distance(a: Time, b: Time) -> Duration {
    Duration::from_nanos(a.as_nanos().abs_diff(b.as_nanos()))
}

fn ground_to_field_from_field_to_robot(
    field_to_robot: Isometry3<Field, Robot>,
    robot_to_ground: Isometry3<Robot, Ground>,
) -> Isometry2<Ground, Field> {
    let robot_to_field = field_to_robot.inverse();
    let ground_to_field = robot_to_field * robot_to_ground.inverse();
    let (_, _, field_to_robot_yaw) = field_to_robot.inner.rotation.euler_angles();
    let yaw = -field_to_robot_yaw;
    let translation = ground_to_field.inner.translation.vector;

    Isometry2::wrap(nalgebra::Isometry2::new(
        nalgebra::vector![translation.x, translation.y],
        yaw,
    ))
}

#[cfg(test)]
mod tests {
    use linear_algebra::IntoTransform;

    use super::*;

    #[test]
    fn ground_to_field_from_field_to_robot_flattens_robot_pose() {
        let robot_to_field = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(1.5, -2.0, 0.4),
            nalgebra::UnitQuaternion::from_euler_angles(0.0, 0.0, 0.7),
        );
        let field_to_robot: Isometry3<Field, Robot> = robot_to_field.inverse().framed_transform();
        let robot_to_ground = Isometry3::identity();

        let ground_to_field = ground_to_field_from_field_to_robot(field_to_robot, robot_to_ground);

        assert!((ground_to_field.translation().x() - 1.5).abs() < 1.0e-6);
        assert!((ground_to_field.translation().y() + 2.0).abs() < 1.0e-6);
        assert!((ground_to_field.orientation().angle() - 0.7).abs() < 1.0e-6);
    }

    #[test]
    fn ground_to_field_from_field_to_robot_ignores_ground_roll_pitch() {
        let robot_to_field = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(1.5, -2.0, 0.4),
            nalgebra::UnitQuaternion::from_euler_angles(0.0, 0.0, 0.7),
        );
        let field_to_robot: Isometry3<Field, Robot> = robot_to_field.inverse().framed_transform();
        let robot_to_ground: Isometry3<Robot, Ground> = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(0.0, 0.0, 0.523),
            nalgebra::UnitQuaternion::from_euler_angles(0.045, 0.047, 0.0),
        )
        .framed_transform();

        let ground_to_field = ground_to_field_from_field_to_robot(field_to_robot, robot_to_ground);

        assert!((ground_to_field.orientation().angle() - 0.7).abs() < 1.0e-6);
    }
}
