use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use std::{boxed::Box, future::Future, pin::Pin};

use color_eyre::Result;

use ros_z::prelude::*;
use ros_z::qos::QosDurability;
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use types::{stereo_camera_info::StereoCameraInfo, stereo_image_pair::StereoImagePair};
use x5_receiver::receiver::{Side, X5Receiver};

const X5_ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 127, 10)), 7654);

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("image_receiver").build().await?;

    let left_image_pub = node.publisher::<Image>("inputs/left_image").build().await?;
    let right_image_pub = node
        .publisher::<Image>("inputs/right_image")
        .build()
        .await?;
    let camera_info_pub = node
        .publisher::<CameraInfo>("inputs/camera_info")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let stereo_camera_info_pub = node
        .publisher::<StereoCameraInfo>("inputs/stereo_camera_info")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let stereo_image_pair_pub = node
        .publisher::<StereoImagePair>("inputs/stereo_image_pair")
        .build()
        .await?;

    let x5_receiver = X5Receiver::new(X5_ADDRESS);
    let camera_info = x5_receiver.wait_for_camera_info().await;
    let stereo_camera_info = StereoCameraInfo {
        left: camera_info.left_camera_info(),
        right: camera_info.right_camera_info(),
    };

    let mut camera_info_timer = node.clock().interval(Duration::from_secs(1));
    let mut stereo_image_pairer = StereoImagePairer::default();
    let mut left_frame_receiver = x5_receiver.subscribe_image(Side::Left);
    let mut right_frame_receiver = x5_receiver.subscribe_image(Side::Right);

    loop {
        let stereo_image_pair = tokio::select! {
            left_image = left_frame_receiver.recv() => {
                let frame_identifier = left_image.header.frame_identifier;
                let image: Image = left_image.clone().into();
                left_image_pub.publish(&image).await?;
                stereo_image_pairer.insert(CameraSide::Left, frame_identifier, image)
            }
            right_image = right_frame_receiver.recv() => {
                let frame_identifier = right_image.header.frame_identifier;
                let image: Image = right_image.clone().into();
                right_image_pub.publish(&image).await?;
                stereo_image_pairer.insert(CameraSide::Right, frame_identifier, image)
            }
            _ = camera_info_timer.tick() => {
                camera_info_pub
                    .publish(&stereo_camera_info.left)
                    .await?;
                stereo_camera_info_pub
                    .publish(&stereo_camera_info)
                    .await?;
                continue
            }
        };

        if !stereo_image_pair_pub.has_subscribers() {
            stereo_image_pairer.clear();
            continue;
        }

        let Some(stereo_image_pair) = stereo_image_pair else {
            continue;
        };
        stereo_image_pair_pub.publish(&stereo_image_pair).await?;
    }
}

#[derive(Clone, Copy)]
enum CameraSide {
    Left,
    Right,
}

#[derive(Default)]
struct StereoImagePairer {
    pending_left: BTreeMap<u32, Image>,
    pending_right: BTreeMap<u32, Image>,
    latest_frame_identifier: u32,
}

impl StereoImagePairer {
    const MAX_UNMATCHED_FRAME_AGE: u32 = 8;

    fn insert(
        &mut self,
        side: CameraSide,
        frame_identifier: u32,
        image: Image,
    ) -> Option<StereoImagePair> {
        self.update_latest_frame_identifier(frame_identifier);

        let (remove_from, insert_in) = match side {
            CameraSide::Left => (&mut self.pending_right, &mut self.pending_left),
            CameraSide::Right => (&mut self.pending_left, &mut self.pending_right),
        };

        let Some(other) = remove_from.remove(&frame_identifier) else {
            insert_in.insert(frame_identifier, image);
            self.expire_old_frames();
            return None;
        };

        self.expire_old_frames();

        let (left, right) = match side {
            CameraSide::Left => (image, other),
            CameraSide::Right => (other, image),
        };

        Some(StereoImagePair {
            frame_identifier,
            left,
            right,
        })
    }

    fn clear(&mut self) {
        self.pending_left.clear();
        self.pending_right.clear();
    }

    fn update_latest_frame_identifier(&mut self, frame_identifier: u32) {
        self.latest_frame_identifier = self.latest_frame_identifier.max(frame_identifier);
    }

    fn expire_old_frames(&mut self) {
        let cutoff = self
            .latest_frame_identifier
            .saturating_sub(Self::MAX_UNMATCHED_FRAME_AGE);
        self.pending_left
            .retain(|frame_identifier, _| *frame_identifier >= cutoff);
        self.pending_right
            .retain(|frame_identifier, _| *frame_identifier >= cutoff);
    }
}
