use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use std::{boxed::Box, future::Future, pin::Pin};

use color_eyre::Result;
use image::RgbImage;

use ros_z::{prelude::*, qos::QosDurability, time::Time};
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use types::{
    stereo_image_pair::StereoImagePair, time_wrapper::TimeWrapper, ycbcr422_image::YCbCr422Image,
};
use x5_receiver::{
    receiver::{Side, X5Receiver},
    types::X5CameraFrame,
};

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
    let ycbcr422_image_pub = node
        .publisher::<TimeWrapper<YCbCr422Image>>("inputs/ycbcr422_image")
        .build()
        .await?;
    let stereo_image_pair_pub = node
        .publisher::<TimeWrapper<StereoImagePair>>("inputs/stereo_image_pair")
        .build()
        .await?;

    let x5_receiver = X5Receiver::new(X5_ADDRESS);
    let camera_info = x5_receiver.wait_for_camera_info().await;

    let mut camera_info_timer = node.clock().interval(Duration::from_secs(1));
    let mut stereo_image_pairer = StereoImagePairer::default();
    let mut left_frame_receiver = x5_receiver.subscribe_image(Side::Left);
    let mut right_frame_receiver = x5_receiver.subscribe_image(Side::Right);

    loop {
        tokio::select! {
            left_image = left_frame_receiver.recv() => {
                let now = node.clock().now();
                let received = ReceivedImage::new(now, left_image);
                handle_left_image(
                    &left_image_pub,
                    &ycbcr422_image_pub,
                    &stereo_image_pair_pub,
                    &mut stereo_image_pairer,
                    received,
                )
                .await?;
            }
            right_image = right_frame_receiver.recv() => {
                let now = node.clock().now();
                let received = ReceivedImage::new(now, right_image);
                handle_right_image(
                    &right_image_pub,
                    &stereo_image_pair_pub,
                    &mut stereo_image_pairer,
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

#[derive(Debug, Clone)]
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

async fn handle_left_image(
    left_image_pub: &Publisher<TimeWrapper<Image>>,
    ycbcr422_image_pub: &Publisher<TimeWrapper<YCbCr422Image>>,
    stereo_image_pair_pub: &Publisher<TimeWrapper<StereoImagePair>>,
    stereo_image_pairer: &mut StereoImagePairer,
    received_image: ReceivedImage,
) -> Result<()> {
    left_image_pub
        .publish(&TimeWrapper {
            time: received_image.image_time,
            inner: received_image.image.clone(),
        })
        .await?;

    publish_ycbcr422_image(ycbcr422_image_pub, received_image.clone()).await?;

    maybe_publish_stereo_image_pair(
        stereo_image_pair_pub,
        stereo_image_pairer,
        CameraSide::Left,
        received_image,
    )
    .await
}

async fn handle_right_image(
    right_image_pub: &Publisher<TimeWrapper<Image>>,
    stereo_image_pair_pub: &Publisher<TimeWrapper<StereoImagePair>>,
    stereo_image_pairer: &mut StereoImagePairer,
    received_image: ReceivedImage,
) -> Result<()> {
    right_image_pub
        .publish(&TimeWrapper {
            time: received_image.image_time,
            inner: received_image.image.clone(),
        })
        .await?;

    maybe_publish_stereo_image_pair(
        stereo_image_pair_pub,
        stereo_image_pairer,
        CameraSide::Right,
        received_image,
    )
    .await
}

async fn publish_ycbcr422_image(
    ycbcr422_image_pub: &Publisher<TimeWrapper<YCbCr422Image>>,
    image: ReceivedImage,
) -> Result<()> {
    let rgb_image: RgbImage = image.image.try_into()?;
    ycbcr422_image_pub
        .publish(&TimeWrapper {
            time: image.image_time,
            inner: (&rgb_image).into(),
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
