use booster::LowState;
use color_eyre::{eyre::bail, Result};
use hulkz::Session;
use image::{error::DecodingError, ImageBuffer, ImageError, RgbImage};
use ros2::sensor_msgs::image::Image;
use yuv::{yuv_nv12_to_rgb, YuvBiPlanarImage, YuvConversionMode, YuvRange, YuvStandardMatrix};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let rec = rerun::RecordingStreamBuilder::new("simple-viewer").spawn()?;

    // let namespace = "HULK10";
    // let session = Session::create(namespace)
    //     .await
    //     .wrap_err("failed to create session")?;



    // let mut stream = session.stream::<Image>("booster/rectified_image").await?;
    //
    // while let Ok(image) = stream.recv_async().await {
    //     let rgb_image = convert_image(&image)?;
    //     rec.log(
    //         "booster/rectified_image",
    //         &rerun::archetypes::Image::from_image(rgb_image)?,
    //     )?;
    // }

    // let mut stream = session
    //     .create_stream::<LowState>("booster/low_state")
    //     .await?;
    //
    // while let Ok(value) = stream.recv_async().await {
    //     let accelerometer = value.imu_state.linear_acceleration;
    //     rec.log(
    //         "booster/imu/accelerometer",
    //         &rerun::archetypes::Scalars::new([
    //             accelerometer.x(),
    //             accelerometer.y(),
    //             accelerometer.z(),
    //         ]),
    //     )?;
    //     let rpy = value.imu_state.roll_pitch_yaw;
    //     rec.log(
    //         "booster/imu/roll_pitch_yaw",
    //         &rerun::archetypes::Scalars::new([rpy.x(), rpy.y(), rpy.z()]),
    //     )?;
    // }

    Ok(())
}

fn convert_image(image: &Image) -> Result<ImageBuffer<image::Rgb<u8>, Vec<u8>>> {
    let y_plane_size = (image.step * image.height) as usize;
    let uv_plane_size = (image.step * image.height / 2) as usize;

    if image.data.len() < y_plane_size + uv_plane_size {
        bail!("NV12: Source buffer is too small for the given dimensions");
    }

    let mut rgb_image = RgbImage::new(image.width, image.height);

    let y_stride = image.step;
    let uv_stride = image.step;
    let rgb_stride = image.width * 3;

    let (y_plane, remaining) = image.data.split_at(y_plane_size);
    let uv_plane = &remaining[..uv_plane_size];

    let yuv_bi_planar_image = YuvBiPlanarImage {
        y_plane,
        y_stride,
        uv_plane,
        uv_stride,
        width: image.width,
        height: image.height,
    };

    yuv_nv12_to_rgb(
        &yuv_bi_planar_image,
        rgb_image.as_flat_samples_mut().as_mut_slice(),
        rgb_stride,
        YuvRange::Limited,
        YuvStandardMatrix::Bt709,
        YuvConversionMode::Balanced,
    )
    .map_err(|e| {
        ImageError::Decoding(DecodingError::from_format_hint(
            image::error::ImageFormatHint::Name(format!("NV12: {e}")),
        ))
    })?;
    Ok(rgb_image)
}
