use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use std::{boxed::Box, future::Future, pin::Pin};

use color_eyre::Result;

use ros_z::prelude::*;
use ros_z::qos::QosDurability;
use ros_z::time::Time;
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use types::{stereo_image_pair::StereoImagePair, time_wrapper::TimeWrapper};
use x5_receiver::receiver::{Side, X5Receiver};
use x5_receiver::types::X5CameraFrame;

const X5_ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 127, 10)), 7654);

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("image_receiver").build().await?;

    let left_image_pub = node
        .publisher::<TimeWrapper<Image>>("inputs/left_image")
        .build()
        .await?;
    let right_image_pub = node
        .publisher::<TimeWrapper<Image>>("inputs/right_image")
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
    let x5_receiver = X5Receiver::new(X5_ADDRESS);
    let camera_info = x5_receiver.wait_for_camera_info().await;

    let mut camera_info_timer = node.clock().interval(Duration::from_secs(1));
    let stereo_image_pair_pub = node
        .publisher::<TimeWrapper<StereoImagePair>>("inputs/stereo_image_pair")
        .build()
        .await?;
    let mut stereo_image_pairer = StereoImagePairer::default();
    let mut left_frame_receiver = x5_receiver.subscribe_image(Side::Left);
    let mut right_frame_receiver = x5_receiver.subscribe_image(Side::Right);

    loop {
        tokio::select! {
            left_image = left_frame_receiver.recv() => {
                let now = node.clock().now();
                let received = ReceivedImage::new(now, left_image);
                handle_image(&left_image_pub, received.clone()).await?;
                maybe_publish_stereo_image_pair(
                    &stereo_image_pair_pub,
                    &mut stereo_image_pairer,
                    CameraSide::Left,
                    received,
                )
                .await?;
            }
            right_image = right_frame_receiver.recv() => {
                let now = node.clock().now();
                let received = ReceivedImage::new(now, right_image);
                handle_image(&right_image_pub, received.clone()).await?;
                maybe_publish_stereo_image_pair(
                    &stereo_image_pair_pub,
                    &mut stereo_image_pairer,
                    CameraSide::Right,
                    received,
                )
                .await?;
            }
            _ = camera_info_timer.tick() => {
                camera_info_pub
                    .publish(&camera_info.left_camera_info())
                    .await?;
            }
        }
    }
}

#[derive(Clone, Debug)]
struct ReceivedImage {
    frame_identifier: u32,
    image_time: Time,
    image: Image,
}

impl ReceivedImage {
    fn new(image_time: Time, frame: X5CameraFrame) -> Self {
        Self {
            frame_identifier: frame.header.frame_identifier,
            image_time,
            image: frame.into(),
        }
    }
}

async fn handle_image(
    image_pub: &Publisher<TimeWrapper<Image>>,
    received_image: ReceivedImage,
) -> Result<()> {
    image_pub
        .publish(&TimeWrapper {
            time: received_image.image_time,
            inner: received_image.image,
        })
        .await?;
    Ok(())
}

async fn maybe_publish_stereo_image_pair(
    stereo_image_pair_pub: &Publisher<TimeWrapper<StereoImagePair>>,
    stereo_image_pairer: &mut StereoImagePairer,
    side: CameraSide,
    image: ReceivedImage,
) -> Result<()> {
    if !stereo_image_pair_pub.has_subscribers() {
        stereo_image_pairer.clear();
        return Ok(());
    }

    let time = image.image_time;
    if let Some(stereo_image_pair) = stereo_image_pairer.insert(side, image) {
        stereo_image_pair_pub
            .publish(&TimeWrapper {
                time,
                inner: stereo_image_pair,
            })
            .await?;
    }

    Ok(())
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

    fn insert(&mut self, side: CameraSide, image: ReceivedImage) -> Option<StereoImagePair> {
        self.update_latest_frame_identifier(image.frame_identifier);

        let (remove_from, insert_in) = match side {
            CameraSide::Left => (&mut self.pending_right, &mut self.pending_left),
            CameraSide::Right => (&mut self.pending_left, &mut self.pending_right),
        };

        let Some(other) = remove_from.remove(&image.frame_identifier) else {
            insert_in.insert(image.frame_identifier, image.image);
            self.expire_old_frames();
            return None;
        };

        self.expire_old_frames();

        let (left, right) = match side {
            CameraSide::Left => (image.image, other),
            CameraSide::Right => (other, image.image),
        };

        Some(StereoImagePair {
            frame_identifier: image.frame_identifier,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn image_with_identifier(frame_identifier: u32) -> ReceivedImage {
        ReceivedImage {
            frame_identifier,
            image_time: Time::zero(),
            image: Image {
                width: frame_identifier,
                ..Default::default()
            },
        }
    }

    #[test]
    fn stereo_image_pairer_publishes_matching_left_and_right_frames() {
        let mut pairer = StereoImagePairer::default();

        assert!(
            pairer
                .insert(CameraSide::Left, image_with_identifier(7))
                .is_none()
        );

        let pair = pairer
            .insert(CameraSide::Right, image_with_identifier(7))
            .expect("matching frames should produce a stereo image pair");

        assert_eq!(pair.frame_identifier, 7);
        assert_eq!(pair.left.width, 7);
        assert_eq!(pair.right.width, 7);
    }

    #[test]
    fn stereo_image_pairer_expires_old_unmatched_frames() {
        let mut pairer = StereoImagePairer::default();
        pairer.insert(CameraSide::Left, image_with_identifier(1));

        for frame_identifier in 2..=10 {
            pairer.insert(CameraSide::Right, image_with_identifier(frame_identifier));
        }

        assert!(
            pairer
                .insert(CameraSide::Right, image_with_identifier(1))
                .is_none()
        );
    }
}
