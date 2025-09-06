from collections.abc import Collection

class SimulationServer:
    def __new__(cls, bind_address: str) -> SimulationServer: ...
    def stop(self) -> None: ...
    def send_low_state(self, low_state: LowState) -> None: ...
    def receive_low_command(self) -> LowCommand | None: ...
    def say_hello(self) -> str: ...

class LowCommand:
    command_type: CommandType
    motor_command: Collection[MotorCommand]

class LowState:
    imu_state: ImuState
    motor_state_serial: Collection[MotorState]
    motor_state_parallel: Collection[MotorState]

class CommandType: ...

class ImuState:
    roll_pitch_yaw: Collection[float]
    angular_velocity: Collection[float]
    linear_acceleration: Collection[float]

class MotorState:
    position: float
    velocity: float
    acceleration: float
    torque: float

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
