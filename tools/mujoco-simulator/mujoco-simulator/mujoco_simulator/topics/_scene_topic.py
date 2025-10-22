import json
from dataclasses import asdict, dataclass
from typing import override

import mujoco
from mujoco import MjData, MjModel
from mujoco_rust_server import SimulationServer

from ._base_topic import SendTopic


@dataclass(kw_only=True, frozen=True)
class BodyState:
    pos: list[float]
    quat: list[float]


@dataclass(kw_only=True, frozen=True)
class SceneState:
    timestamp: float
    bodies: dict[str, BodyState]


class SceneStateTopic(SendTopic):
    name = "scene_state"

    @override
    def compute(self, *, model: MjModel, data: MjData) -> SceneState:
        bodies = {}

        for i in range(model.nbody):
            name = mujoco.mj_id2name(model, mujoco.mjtObj.mjOBJ_BODY.value, i)
            pos = data.xpos[i].tolist()
            quat = data.xquat[i].tolist()  # (w, x, y, z)
            bodies[name] = {"pos": pos, "quat": quat}

        return SceneState(timestamp=data.time, bodies=bodies)

    @override
    def publish(
        self, *, server: SimulationServer, model: MjModel, data: MjData
    ) -> None:
        state = self.compute(model=model, data=data)
        server.update_scene_state(json.dumps(asdict(state)))
