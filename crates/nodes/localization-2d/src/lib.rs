use std::{boxed::Box, future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use coordinate_systems::{Field, Ground, Robot};
use linear_algebra::{IntoTransform, Isometry2, Isometry3};
use ros_z::{
    cache::Cache,
    context::Context,
    parameter::NodeParametersExt,
    qos::{QosDurability, QosProfile},
    time::Time,
};
use types::{
    localization::{LocalizationEstimate, LocalizationStatus, ground_to_field_from_field_to_robot},
    parameters::Localization2dParameters,
    time_wrapper::TimeWrapper,
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("localization2d").build().await?;
    let parameters = node.bind_parameter_as::<Localization2dParameters>("localization2d")?;
    let capacities = parameters.snapshot().typed.clone();
    let startup = capacities.clone();
    parameters.add_validation_hook(move |candidate| {
        if candidate.max_ground_time_distance.is_zero()
            || candidate.ground_cache_capacity == 0
            || candidate.status_cache_capacity == 0
        {
            return Err("localization2d gap and capacities must be positive".into());
        }
        if candidate.ground_cache_capacity != startup.ground_cache_capacity
            || candidate.status_cache_capacity != startup.status_cache_capacity
        {
            return Err("localization2d cache capacity changes require restart".into());
        }
        Ok(())
    })?;

    let localization_subscriber = node
        .subscriber::<LocalizationEstimate>("localization/estimate")
        .build()
        .await?;
    let robot_to_ground_cache = node
        .subscriber::<TimeWrapper<Option<Isometry3<Robot, Ground>>>>("robot_to_ground")
        .cache(capacities.ground_cache_capacity)
        .with_stamp(|wrapper| wrapper.time)
        .build()
        .await?;
    let status = node
        .subscriber::<LocalizationStatus>("localization/status")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(capacities.status_cache_capacity)
        .build()
        .await?;

    let ground_to_field_publisher = node
        .publisher::<Isometry2<Ground, Field>>("ground_to_field")
        .build()
        .await?;

    loop {
        let localization = localization_subscriber.recv().await?;
        if status.get_latest().is_none_or(|status| {
            status.epoch != localization.epoch || status.generation != localization.generation
        }) {
            continue;
        }
        let time = localization.time;
        let Some(robot_to_field) = localization.robot_to_field else {
            continue;
        };

        let max_age = parameters.snapshot().typed.max_ground_time_distance;
        let Some(robot_to_ground) = fresh_robot_to_ground_at(&robot_to_ground_cache, time, max_age)
        else {
            continue;
        };

        ground_to_field_publisher
            .publish_with_source_time(
                &ground_to_field_from_field_to_robot(
                    robot_to_field
                        .pose
                        .inner
                        .cast()
                        .inverse()
                        .framed_transform(),
                    robot_to_ground,
                ),
                time,
            )
            .await?;
    }
}

fn fresh_robot_to_ground_at(
    robot_to_ground_cache: &Cache<TimeWrapper<Option<Isometry3<Robot, Ground>>>>,
    time: Time,
    max_age: Duration,
) -> Option<Isometry3<Robot, Ground>> {
    robot_to_ground_cache
        .get_nearest_with_stamp(time)
        .filter(|(stamp, _)| stamp.abs_diff(time) <= max_age)
        .and_then(|(_, transform)| transform.inner)
}
