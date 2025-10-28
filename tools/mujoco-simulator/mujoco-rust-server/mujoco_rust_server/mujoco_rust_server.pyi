from collections.abc import Collection
from enum import Enum, auto

class SimulationServer:
    def __new__(cls, bind_address: str) -> SimulationServer: ...
    async def stop(self) -> None: ...
    async def next_task(self, current_time: float) -> ControllerTask: ...
    def register_scene(self, scene: bytes) -> None: ...
    def update_scene_state(self, state: bytes) -> None: ...

class ControllerTask:
    name: TaskName

class TaskName(Enum):
    ApplyLowCommand = auto()
    RequestLowState = auto()
    RequestRGBDSensors = auto()
    StepSimulation = auto()
    Reset = auto()

class booster_types:
    class LowCommand:
        command_type: booster_types.CommandType
        motor_command: Collection[booster_types.MotorCommand]

        def __new__(
            cls,
            command_type: booster_types.CommandType,
            motor_command: Collection[booster_types.MotorCommand],
        ) -> booster_types.LowCommand: ...

    class LowState:
        imu_state: booster_types.ImuState
        motor_state_serial: Collection[booster_types.MotorState]
        motor_state_parallel: Collection[booster_types.MotorState]

        def __new__(
            cls,
            imu_state: booster_types.ImuState,
            motor_state_serial: Collection[booster_types.MotorState],
            motor_state_parallel: Collection[booster_types.MotorState],
        ) -> booster_types.LowState: ...

    class CommandType(Enum):
        Parallel = auto()
        Serial = auto()

    class ImuState:
        def __new__(
            cls,
            roll_pitch_yaw: Collection[float],
            angular_velocity: Collection[float],
            linear_acceleration: Collection[float],
        ) -> booster_types.ImuState: ...

    class MotorState:
        position: float
        velocity: float
        acceleration: float
        torque: float

        def __new__(
            cls,
            position: float,
            velocity: float,
            acceleration: float,
            torque: float,
        ) -> booster_types.MotorState: ...

    class MotorCommand:
        position: float
        velocity: float
        torque: float
        kp: float
        kd: float
        weight: float

        def __new__(
            cls,
            position: float,
            velocity: float,
            torque: float,
            kp: float,
            kd: float,
            weight: float,
        ) -> booster_types.MotorCommand: ...

class zed_types:
    class RGBDSensors:
        def __new__(
            cls, rgb: bytes, depth: bytes, height: int, width: int
        ) -> zed_types.RGBDSensors: ...
