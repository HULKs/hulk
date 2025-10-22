from datetime import timedelta
from typing import override

from mujoco import MjData, MjModel
from mujoco_rust_server import SimulationServer
from mujoco_rust_server.zed_types import RGBDSensors

from mujoco_simulator._camera_render import CameraRenderer

from ._base_topic import SendTopic


class CameraTopic(SendTopic):
    name = "camera"
    camera_encoder: CameraRenderer

    def __init__(
        self,
        update_interval: timedelta,
        model: MjModel,
        camera_name: str = "camera",
    ) -> None:
        super().__init__(update_interval)
        self.camera_encoder = CameraRenderer(
            model=model, camera_name=camera_name
        )

    @override
    def compute(self, *, model: MjModel, data: MjData) -> RGBDSensors:
        image = self.camera_encoder.render(data)
        return RGBDSensors(
            data.time,
            image.rgb.flatten(),
            image.depth.flatten(),
            image.height(),
            image.width(),
        )

    @override
    def publish(
        self, *, server: SimulationServer, model: MjModel, data: MjData
    ) -> None:
        server.send_camera_frame(
            data.time, self.compute(model=model, data=data)
        )
