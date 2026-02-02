from collections.abc import Collection
from enum import Enum, auto

class LowCommand:
    command_type: CommandType
    motor_commands: Collection[MotorCommand]

    def __new__(
        cls,
        command_type: CommandType,
        motor_command: Collection[MotorCommand],
    ) -> LowCommand: ...

class LowState:
    imu_state: ImuState
    motor_state_parallel: Collection[MotorState]
    motor_state_serial: Collection[MotorState]

    def __new__(
        cls,
        imu_state: ImuState,
        motor_state_parallel: Collection[MotorState],
        motor_state_serial: Collection[MotorState],
    ) -> LowState: ...

class CommandType(Enum):
    Parallel = auto()
    Serial = auto()

class ImuState:
    def __new__(
        cls,
        roll_pitch_yaw: Collection[float],
        angular_velocity: Collection[float],
        linear_acceleration: Collection[float],
    ) -> ImuState: ...

class MotorState:
    mode: int
    position: float
    velocity: float
    acceleration: float
    torque: float
    temperature: int
    lost: int
    reserve: tuple[int, int]

    def __new__(
        cls,
        mode: int,
        position: float,
        velocity: float,
        acceleration: float,
        torque: float,
        temperature: int,
        lost: int,
        reserve: tuple[int, int],
    ) -> MotorState: ...

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
    ) -> MotorCommand: ...
