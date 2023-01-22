use types::hardware::Interface;

pub struct Interfake {}

impl Interface for Interfake {
    fn read_from_microphones(&self) -> color_eyre::Result<types::samples::Samples> {
        unimplemented!()
    }

    fn get_now(&self) -> std::time::SystemTime {
        unimplemented!()
    }

    fn get_ids(&self) -> types::hardware::Ids {
        unimplemented!()
    }

    fn read_from_sensors(&self) -> color_eyre::Result<types::SensorData> {
        unimplemented!()
    }

    fn write_to_actuators(
        &self,
        positions: types::Joints,
        stiffnesses: types::Joints,
        leds: types::Leds,
    ) -> color_eyre::Result<()> {
        unimplemented!()
    }

    fn read_from_network(&self) -> color_eyre::Result<types::messages::IncomingMessage> {
        unimplemented!()
    }

    fn write_to_network(
        &self,
        message: types::messages::OutgoingMessage,
    ) -> color_eyre::Result<()> {
        unimplemented!()
    }

    fn read_from_camera(
        &self,
        camera_position: types::CameraPosition,
    ) -> color_eyre::Result<types::image::Image> {
        unimplemented!()
    }
}
