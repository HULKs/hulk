import mujoco
from _typeshed import Incomplete
from collections.abc import Sequence
from numpy import typing as npt

class Rollout:
    nthread: Incomplete
    rollout_: Incomplete
    def __init__(self, *, nthread: int | None = None) -> None: ...
    def __enter__(self): ...
    def __exit__(self, exc_type: type[BaseException] | None, exc_val: BaseException | None, exc_tb: types.TracebackType | None) -> None: ...
    def close(self) -> None: ...
    def rollout(self, model: mujoco.MjModel | Sequence[mujoco.MjModel], data: mujoco.MjData | Sequence[mujoco.MjData], initial_state: npt.ArrayLike, control: npt.ArrayLike | None = None, *, control_spec: int = ..., skip_checks: bool = False, nstep: int | None = None, initial_warmstart: npt.ArrayLike | None = None, state: npt.ArrayLike | None = None, sensordata: npt.ArrayLike | None = None, chunk_size: int | None = None): ...

persistent_rollout: Incomplete

def shutdown_persistent_pool() -> None: ...
def rollout(model: mujoco.MjModel | Sequence[mujoco.MjModel], data: mujoco.MjData | Sequence[mujoco.MjData], initial_state: npt.ArrayLike, control: npt.ArrayLike | None = None, *, control_spec: int = ..., skip_checks: bool = False, nstep: int | None = None, initial_warmstart: npt.ArrayLike | None = None, state: npt.ArrayLike | None = None, sensordata: npt.ArrayLike | None = None, chunk_size: int | None = None, persistent_pool: bool = False): ...
