use byteorder::{BigEndian, LittleEndian};
use bytes::Bytes;
use cdr_encoding::CdrDeserializer;
use color_eyre::eyre::{eyre, Context, Result};
use flume::bounded;
use ros2::sensor_msgs::camera_info::CameraInfo;
use serde::Deserialize;

pub fn setup_logger() -> Result<(), fern::InitError> {
    env_logger::init();
    Ok(())
}

#[derive(Deserialize)]
struct DDSDataWrapper {
    representation_identifier: [u8; 2],
    representation_options: [u8; 2],
    bytes: Bytes,
}

impl DDSDataWrapper {
    fn new(bytes: &[u8]) -> Self {
        Self {
            representation_identifier: bytes[0..2].try_into().unwrap(),
            representation_options: bytes[2..4].try_into().unwrap(),
            bytes: bytes[4..].to_owned().into(),
        }
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    setup_logger()?;
    let session = zenoh::open(zenoh::Config::default()).await.unwrap();

    log::info!("Creating subscriber...");
    let subscriber = session
        .declare_subscriber("StereoNetNode/camera_info")
        .with(bounded(32))
        .await
        .unwrap();
    log::info!("Created subscriber...");
    loop {
        log::info!("Receiving...");
        let sample = subscriber
            .recv_async()
            .await
            .map_err(|err| eyre!(err.to_string()))
            .wrap_err("recv failed")?;
        println!("Received: {} {:?}", sample.key_expr(), sample.payload());
        let ddsdata_wrapper = DDSDataWrapper::new(&sample.payload().to_bytes());
        let message: Option<CameraInfo> = match ddsdata_wrapper.representation_identifier {
            [0x00, 0x01] => {
                let mut deserializer = CdrDeserializer::<LittleEndian>::new(&ddsdata_wrapper.bytes);
                Some(serde::de::Deserialize::deserialize(&mut deserializer).unwrap())
            }
            [0x00, 0x00] => {
                let mut deserializer = CdrDeserializer::<BigEndian>::new(&ddsdata_wrapper.bytes);
                Some(serde::de::Deserialize::deserialize(&mut deserializer).unwrap())
            }
            _ => None,
        };

        println!("{:?}", message);
    }
}
