from dataclasses import dataclass
from typing import Self

import mujoco as mj
import numpy as np
import numpy.typing as npt
import optuna


@dataclass
class ActuatorParameters:
    gain_prm: npt.NDArray[np.float64]
    dyn_prm: npt.NDArray[np.float64]
    bias_prm: npt.NDArray[np.float64]

    @classmethod
    def suggest_position_actuator(
        cls,
        trial: optuna.Trial | optuna.trial.FrozenTrial,
        name: str,
    ) -> Self:
        kp = trial.suggest_float(f"{name}_kp", low=0.0, high=100.0)
        kv = trial.suggest_float(f"{name}_kv", low=0.0, high=100.0)

        return cls(
            gain_prm=np.array([kp, 0.0, 0.0], dtype=np.float64),
            dyn_prm=np.array([1.0, 0.0, 0.0], dtype=np.float64),
            bias_prm=np.array([0.0, -kp, -kv], dtype=np.float64),
        )

    @classmethod
    def suggest_from_trial(
        cls,
        trial: optuna.Trial,
        name: str,
    ) -> Self:
        """Suggest actuator parameters from a trial.

        Args:
            trial: The Optuna trial.
            name: The name of the actuator.

        Returns:
            The suggested actuator parameters.

        """
        gain_prm = [
            trial.suggest_float(
                f"{name}_gain_prm_{i}",
                low=-10.0,
                high=10.0,
            )
            for i in range(3)
        ]
        dyn_prm = [
            trial.suggest_float(
                f"{name}_dyn_prm_{i}",
                low=-10.0,
                high=10.0,
            )
            for i in range(3)
        ]
        bias_prm = [
            trial.suggest_float(
                f"{name}_bias_prm_{i}",
                low=-10.0,
                high=10.0,
            )
            for i in range(3)
        ]
        return cls(
            gain_prm=np.array(gain_prm, dtype=np.float64),
            dyn_prm=np.array(dyn_prm, dtype=np.float64),
            bias_prm=np.array(bias_prm, dtype=np.float64),
        )

    def populate_actuator(
        self,
        actuator: mj.MjsActuator,
    ) -> None:
        """Populate the actuator with the parameters.

        Args:
            actuator: The actuator to populate.

        """
        actuator.gainprm[:3] = self.gain_prm
        actuator.dynprm[:3] = self.dyn_prm
        actuator.biasprm[:3] = self.bias_prm
