use std::{boxed::Box, future::Future, pin::Pin, sync::Arc};

use color_eyre::Result;
use coordinate_systems::{Field, Ground, Robot};
use linear_algebra::{Isometry2, Isometry3};
use ros_z::context::Context;
use types::{localization::ground_to_field_from_field_to_robot, time_wrapper::TimeWrapper};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("localization2d").build().await?;

    let localization_subscriber = node
        .subscriber::<TimeWrapper<Option<Isometry3<Field, Robot>>>>("localization/timestamped")
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

        let Some(robot_to_ground) = robot_to_ground_cache
            .get_nearest(time)
            .and_then(|transform| transform.inner)
        else {
            continue;
        };

        ground_to_field_publisher
            .publish(&ground_to_field_from_field_to_robot(
                field_to_robot,
                robot_to_ground,
            ))
            .await?;
    }
}

#[cfg(test)]
mod tests {
    use linear_algebra::IntoTransform;

    use super::*;

    #[test]
    fn ground_to_field_from_field_to_robot_flattens_pose() {
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
}
