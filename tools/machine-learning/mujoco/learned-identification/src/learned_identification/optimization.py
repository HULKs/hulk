import mujoco as mj
import numpy as np
import numpy.typing as npt
import optuna

from .actuator import ActuatorParameters
from .simulation import simulate_recording


class SimulationLengthError(Exception):
    def __init__(self) -> None:
        super().__init__(
            "The number of simulated sensor data points does not match the "
            "number of recorded sensor data points",
        )


def populate_actuators(
    spec: mj.MjSpec,
    trial: optuna.Trial | optuna.trial.FrozenTrial,
) -> None:
    for actuator in spec.actuators:
        parameters = ActuatorParameters.suggest_position_actuator(
            trial,
            actuator.name,
        )
        parameters.populate_actuator(actuator)


def objective(
    trial: optuna.Trial | optuna.trial.FrozenTrial,
    spec: mj.MjSpec,
    recorded_actuators: npt.NDArray[np.float64],
    recorded_sensors: npt.NDArray[np.float64],
    *,
    video_path: str | None = None,
) -> float:
    populate_actuators(spec, trial)
    simulated_sensor_data = simulate_recording(
        spec,
        recorded_actuators,
        video_path=video_path,
    )
    if len(simulated_sensor_data) != len(recorded_sensors):
        raise SimulationLengthError
    squared_error = (simulated_sensor_data - recorded_sensors) ** 2
    return squared_error.sum() / len(recorded_actuators)
