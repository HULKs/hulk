use ros_z::Message;
use ros2::sensor_msgs::image::Image;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
pub struct StereoImagePair {
    pub frame_identifier: u32,
    pub left: Image,
    pub right: Image,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros_z::{
        CdrCodec,
        message::{WireDecoder, WireEncoder},
    };
    use ros2::std_msgs::header::Header;

    #[derive(Serialize, Deserialize, Message)]
    #[message(name = "ros2::sensor_msgs::image::Image")]
    struct LegacyImage {
        header: Header,
        height: u32,
        width: u32,
        encoding: String,
        is_bigendian: u8,
        step: u32,
        data: Vec<u8>,
    }

    #[derive(Serialize, Deserialize, Message)]
    #[message(name = "types::stereo_image_pair::StereoImagePair")]
    struct LegacyStereo {
        frame_identifier: u32,
        left: LegacyImage,
        right: LegacyImage,
    }

    #[test]
    fn nested_images_preserve_wire_schema_and_pixel_owners() {
        assert_eq!(Image::schema(), LegacyImage::schema());
        assert_eq!(Image::schema_hash(), LegacyImage::schema_hash());
        assert_eq!(StereoImagePair::schema_hash(), LegacyStereo::schema_hash());
        let image = Image {
            height: 2,
            width: 4,
            step: 4,
            encoding: "nv12".into(),
            data: vec![42; 12].into(),
            ..Default::default()
        };
        let stereo = StereoImagePair {
            frame_identifier: 7,
            left: image.clone(),
            right: image,
        };
        let pixels = stereo.left.data.as_ptr();
        let mut old = ros_z::message::CDR_HEADER_LE.to_vec();
        let mut serializer = ros_z::__private::ros_z_cdr::SerdeCdrSerializer::<
            ros_z::__private::ros_z_cdr::LittleEndian,
        >::new(&mut old);
        stereo.serialize(&mut serializer).unwrap();
        let native = CdrCodec::<StereoImagePair>::serialize(&stereo).unwrap();
        assert_eq!(native, old);
        let legacy = CdrCodec::<LegacyStereo>::deserialize(&native).unwrap();
        assert_eq!(legacy.left.data, stereo.left.data.as_slice());
        assert_eq!(
            CdrCodec::<LegacyStereo>::serialize(&legacy).unwrap(),
            native
        );
        let payload = CdrCodec::<StereoImagePair>::serialize_to_zbuf(&stereo)
            .unwrap()
            .into();
        let decoded = CdrCodec::<StereoImagePair>::deserialize_payload(&payload).unwrap();
        assert_eq!(decoded.left.data.as_ptr(), pixels);
        assert_eq!(decoded.right.data.as_ptr(), pixels);
        drop(stereo);
        drop(payload);
        assert_eq!(decoded.left.data.as_slice(), &[42; 12]);
    }
}
