use std::sync::Arc;

use color_eyre::{Result, eyre::WrapErr as _};
use eframe::egui::Context as EguiContext;
use ros_z::prelude::ContextBuilder;
use ros_z_debug::{TopicObserver, TopicObserverOptions};
use ros_z_streams::{CreateFutureQueue, QueueEvent};
use ros2::sensor_msgs::image::Image as RosImage;
use tokio::runtime::Runtime;
use types::{
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
};

use crate::{
    cli::Arguments,
    state::{CameraFrame, ConnectionStatus, SharedState, StreamStatus, ViewerState},
};

mod camera;
mod debug;
mod topics;

pub(crate) use topics::CAMERA_IMAGE_TOPIC;

use self::{
    camera::decode_camera_frame,
    debug::DebugSubscriptions,
    topics::{DEBUG_REFRESH_INTERVAL, DETECTED_OBJECTS_SAFETY_LAG, DETECTED_OBJECTS_TOPIC},
};

pub(crate) fn spawn(
    runtime: &Runtime,
    arguments: Arguments,
    state: SharedState,
    egui_context: EguiContext,
) {
    runtime.spawn(async move {
        if let Err(error) = run(arguments, state.clone(), egui_context.clone()).await {
            update_state(&state, &egui_context, |state| {
                state.connection = ConnectionStatus::Error(format!("{error:#}"));
            });
        }
    });
}

async fn run(arguments: Arguments, state: SharedState, egui_context: EguiContext) -> Result<()> {
    update_state(&state, &egui_context, |state| {
        state.connection = ConnectionStatus::Connecting;
    });

    let router_display = arguments.router_display();
    let mut builder = ContextBuilder::default().with_namespace(arguments.namespace());
    if let Some(router) = arguments.router.clone() {
        builder = builder.with_mode("client").with_connect_endpoints([router]);
    }

    let context = builder.build().await.wrap_err_with(|| {
        format!(
            "failed to connect to Zenoh router {router_display}; make sure zenohd is running and listening on that address, or use tcp/127.0.0.1:7447 when running on the robot or through an SSH port forward"
        )
    })?;
    let node = Arc::new(
        context
            .create_node("robot_viewer")
            .without_schema_service()
            .build()
            .await?,
    );
    let debug_observer = TopicObserver::new(
        Arc::clone(&node),
        TopicObserverOptions::with_namespace(arguments.namespace())?,
    );
    let mut debug_subscriptions = DebugSubscriptions::build(&debug_observer)?;

    let camera = node
        .subscriber::<TimeWrapper<RosImage>>(CAMERA_IMAGE_TOPIC)
        .build()
        .await?;
    let mut objects = node
        .create_future_subscriber::<Vec<Object<RobocupObjectLabel>>>(
            DETECTED_OBJECTS_TOPIC,
            DETECTED_OBJECTS_SAFETY_LAG,
        )
        .await?;

    update_state(&state, &egui_context, |state| {
        state.connection = ConnectionStatus::Subscribed;
        update_publisher_count(&mut state.camera_status, camera.publisher_count());
        update_publisher_count(&mut state.objects_status, objects.publisher_count());
    });

    let mut refresh_interval = tokio::time::interval(DEBUG_REFRESH_INTERVAL);
    loop {
        tokio::select! {
            _ = refresh_interval.tick() => {
                update_state(&state, &egui_context, |state| {
                    debug_subscriptions.refresh(state);
                    update_publisher_count(&mut state.camera_status, camera.publisher_count());
                    update_publisher_count(&mut state.objects_status, objects.publisher_count());
                });
            }
            message = camera.recv() => match message {
                Ok(image) => {
                    let time = image.time;
                    match decode_camera_frame(image.inner) {
                        Ok(frame) => update_state(&state, &egui_context, |state| {
                            state.camera_sequence += 1;
                            state.push_camera_frame(time, CameraFrame {
                                sequence: state.camera_sequence,
                                ..frame
                            });
                            state.camera_status.mark_live(camera.publisher_count());
                        }),
                        Err(error) => update_state(&state, &egui_context, |state| {
                            state.camera_status.mark_error(camera.publisher_count(), format!("{error:#}"));
                        }),
                    }
                }
                Err(error) => update_state(&state, &egui_context, |state| {
                    state.camera_status.mark_error(camera.publisher_count(), format!("{error:#}"));
                }),
            },
            message = objects.recv() => match message {
                Ok(QueueEvent::Data(time, message)) => update_state(&state, &egui_context, |state| {
                    state.push_detected_objects(time, message);
                    state.objects_status.mark_live(objects.publisher_count());
                }),
                Ok(QueueEvent::Announcement) => {}
                Err(error) => update_state(&state, &egui_context, |state| {
                    state.objects_status.mark_error(objects.publisher_count(), format!("{error:#}"));
                }),
            },
        }
    }
}

fn update_publisher_count(status: &mut StreamStatus, publisher_count: usize) {
    status.update_publishers(publisher_count);
}

fn update_state(
    state: &SharedState,
    egui_context: &EguiContext,
    update: impl FnOnce(&mut ViewerState),
) {
    update(
        &mut state
            .lock()
            .expect("viewer state lock should not be poisoned"),
    );
    egui_context.request_repaint();
}
