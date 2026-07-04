use std::sync::Arc;
use std::{boxed::Box, future::Future, pin::Pin};

use color_eyre::{
    Result,
    eyre::{Context as _, eyre},
};

use booster::Odometer;
use coordinate_systems::{Ground, Odometry};
use linear_algebra::{Pose2, point};
use ros_z::prelude::*;
use ros_z_streams::CreateAnnouncingPublisher;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("odometer_bridge").build().await?;

    let zenoh_session = ctx.session();

    let odometer_sub = zenoh_session
        .declare_subscriber("rt/odometer_state")
        .await
        .map_err(|error| eyre!("{error}"))?;
    let odometer_pub = node
        .announcing_publisher::<Pose2<Odometry>>("inputs/booster_odometry")
        .await?;
    let mut origin_at_start = None;

    loop {
        tokio::select! {
            odometer = odometer_sub.recv_async() => {
                let odometer = odometer.map_err(|error| eyre!("{error}"))?;

                let odometer: Odometer = cdr::deserialize(&odometer.payload().to_bytes())
                    .wrap_err("deserialization failed")?;
                let raw_odometry_pose = pose_from_booster_odometer(odometer);
                let origin = origin_at_start.get_or_insert(raw_odometry_pose);
                let odometry_pose = pose_relative_to_origin(*origin, raw_odometry_pose);

                odometer_pub
                    .announce(node.clock().now())
                    .await?
                    .publish(&odometry_pose)
                    .await?;
            }
        }
    }
}

fn pose_from_booster_odometer(odometer: Odometer) -> Pose2<Odometry> {
    Pose2::new(point![<Odometry>, odometer.x, odometer.y], odometer.theta)
}

fn pose_relative_to_origin(origin: Pose2<Odometry>, pose: Pose2<Odometry>) -> Pose2<Odometry> {
    let origin_to_odometry = origin.as_transform::<Ground>();
    let pose_to_odometry = pose.as_transform::<Ground>();
    let pose_to_origin = origin_to_odometry.inverse() * pose_to_odometry;

    Pose2::wrap(pose_to_origin.inner)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use booster::Odometer;

    use super::{pose_from_booster_odometer, pose_relative_to_origin};

    #[test]
    fn converts_booster_odometer_to_typed_pose() {
        let pose = pose_from_booster_odometer(Odometer {
            x: 1.2,
            y: -0.3,
            theta: 0.4,
        });

        assert_relative_eq!(pose.position().x(), 1.2, epsilon = 1.0e-6);
        assert_relative_eq!(pose.position().y(), -0.3, epsilon = 1.0e-6);
        assert_relative_eq!(pose.orientation().angle(), 0.4, epsilon = 1.0e-6);
    }

    #[test]
    fn first_booster_pose_maps_to_identity() {
        let origin = pose_from_booster_odometer(Odometer {
            x: 1.2,
            y: -0.3,
            theta: 0.4,
        });

        let pose = pose_relative_to_origin(origin, origin);

        assert_relative_eq!(pose.position().x(), 0.0, epsilon = 1.0e-6);
        assert_relative_eq!(pose.position().y(), 0.0, epsilon = 1.0e-6);
        assert_relative_eq!(pose.orientation().angle(), 0.0, epsilon = 1.0e-6);
    }

    #[test]
    fn later_booster_pose_is_relative_to_startup_origin() {
        let origin = pose_from_booster_odometer(Odometer {
            x: 1.0,
            y: 2.0,
            theta: 0.0,
        });
        let current = pose_from_booster_odometer(Odometer {
            x: 1.5,
            y: 1.25,
            theta: 0.2,
        });

        let pose = pose_relative_to_origin(origin, current);

        assert_relative_eq!(pose.position().x(), 0.5, epsilon = 1.0e-6);
        assert_relative_eq!(pose.position().y(), -0.75, epsilon = 1.0e-6);
        assert_relative_eq!(pose.orientation().angle(), 0.2, epsilon = 1.0e-6);
    }

    #[test]
    fn startup_yaw_rotates_relative_translation() {
        let origin = pose_from_booster_odometer(Odometer {
            x: 1.0,
            y: 2.0,
            theta: std::f32::consts::FRAC_PI_2,
        });
        let current = pose_from_booster_odometer(Odometer {
            x: 1.0,
            y: 3.0,
            theta: std::f32::consts::FRAC_PI_2,
        });

        let pose = pose_relative_to_origin(origin, current);

        assert_relative_eq!(pose.position().x(), 1.0, epsilon = 1.0e-6);
        assert_relative_eq!(pose.position().y(), 0.0, epsilon = 1.0e-6);
        assert_relative_eq!(pose.orientation().angle(), 0.0, epsilon = 1.0e-6);
    }
}
