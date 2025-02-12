from dataclasses import dataclass

import mujoco
import numpy as np
from numpy.typing import NDArray

from nao_interface.joints import Joints, JointsView
from nao_interface.ring_buffer import RingBuffer


@dataclass
class SensorBuffers:
    positions: RingBuffer
    left_fsr: RingBuffer
    right_fsr: RingBuffer
    gyroscope: RingBuffer
    accelerometer: RingBuffer


class Nao:
    def __init__(
        self,
        model: mujoco.MjModel,
        data: mujoco.MjData,
        fsr_scale: float = 1.0,
        position_sensor_delay: int = 0,
        fsr_sensor_delay: int = 0,
        gyroscope_sensor_delay: int = 0,
        accelerometer_sensor_delay: int = 0,
    ) -> None:
        self.model = model
        self.data = data

        self.fsr_scale = fsr_scale

        self.joint_positions = JointsView(
            lambda name: self.data.joint(name).qpos,
            lambda name, value: self.data.joint(name).__setattr__(
                "qpos", value
            ),
        )
        self.actuator_control = JointsView(
            lambda name: self.data.actuator(name).ctrl[0],
            lambda name, value: self.data.actuator(name).__setattr__(
                "ctrl",
                value,
            ),
        )
        self.position_sensors = JointsView(
            lambda name: self.data.sensor(name).data[0],
            lambda name, value: self.data.sensor(name).__setattr__(
                "data",
                value,
            ),
        )

        self._sensor_buffers = SensorBuffers(
            positions=RingBuffer(
                position_sensor_delay + 1,
                self._read_positions(),
            ),
            left_fsr=RingBuffer(
                fsr_sensor_delay + 1,
                self._read_left_fsr_values(),
            ),
            right_fsr=RingBuffer(
                fsr_sensor_delay + 1,
                self._read_right_fsr_values(),
            ),
            gyroscope=RingBuffer(
                gyroscope_sensor_delay + 1,
                self._read_gyroscope(),
            ),
            accelerometer=RingBuffer(
                accelerometer_sensor_delay + 1,
                self._read_accelerometer(),
            ),
        )

    def set_transform(
        self,
        position: NDArray[np.floating],
        quaternion: NDArray[np.floating],
    ) -> None:
        nao = self.data.body("Nao")
        nao.xpos = position
        nao.xquat = quaternion

    def reset(self, positions: Joints) -> None:
        mujoco.mj_resetData(self.model, self.data)

        self.joint_positions.set_from_joints(positions)
        self.actuator_control.set_from_joints(positions)

        mujoco.mj_forward(self.model, self.data)

    def update_sensors(self) -> None:
        self._sensor_buffers.positions.push(self._read_positions())
        self._sensor_buffers.left_fsr.push(self._read_left_fsr_values())
        self._sensor_buffers.right_fsr.push(self._read_right_fsr_values())
        self._sensor_buffers.gyroscope.push(self._read_gyroscope())
        self._sensor_buffers.accelerometer.push(self._read_accelerometer())

    def _read_positions(self) -> NDArray[np.floating]:
        return self.position_sensors.to_numpy()

    def _read_left_fsr_values(self) -> NDArray[np.floating]:
        return self.fsr_scale * np.array(
            [
                self.data.sensor(
                    "force_sensitive_resistors.left.front_left",
                ).data[0],
                self.data.sensor(
                    "force_sensitive_resistors.left.front_right",
                ).data[0],
                self.data.sensor(
                    "force_sensitive_resistors.left.rear_left",
                ).data[0],
                self.data.sensor(
                    "force_sensitive_resistors.left.rear_right",
                ).data[0],
            ],
        )

    def _read_right_fsr_values(self) -> NDArray[np.floating]:
        return self.fsr_scale * np.array(
            [
                self.data.sensor(
                    "force_sensitive_resistors.right.front_left",
                ).data[0],
                self.data.sensor(
                    "force_sensitive_resistors.right.front_right",
                ).data[0],
                self.data.sensor(
                    "force_sensitive_resistors.right.rear_left",
                ).data[0],
                self.data.sensor(
                    "force_sensitive_resistors.right.rear_right",
                ).data[0],
            ],
        )

    def _read_gyroscope(self) -> NDArray[np.floating]:
        return self.data.sensor("gyroscope").data

    def _read_accelerometer(self) -> NDArray[np.floating]:
        return self.data.sensor("accelerometer").data

    def position_encoders(self) -> NDArray[np.floating]:
        return self._sensor_buffers.positions.left()

    def left_fsr(self) -> NDArray[np.floating]:
        return self._sensor_buffers.left_fsr.left()

    def right_fsr(self) -> NDArray[np.floating]:
        return self._sensor_buffers.right_fsr.left()

    def gyroscope(self) -> NDArray[np.floating]:
        return self._sensor_buffers.gyroscope.left()

    def accelerometer(self) -> NDArray[np.floating]:
        return self._sensor_buffers.accelerometer.left()
