from dataclasses import replace
from typing import Literal

from mjlab.envs import ManagerBasedRlEnvCfg
from mjlab.scene import SceneCfg
from mjlab.sensor import ContactMatch, ContactSensorCfg
from mjlab.sim import MujocoCfg, SimulationCfg
import mjlab.terrains as terrain_gen
from mjlab.terrains import TerrainEntityCfg
from mjlab.terrains.config import ROUGH_TERRAINS_CFG
from mjlab.terrains.terrain_generator import TerrainGeneratorCfg
from mjlab.viewer import ViewerConfig

from konerl.k1_config import get_k1_robot_cfg

from .actions import make_actions_cfg, make_commands_cfg
from .curriculum import make_curriculum_cfg
from .observations import make_observation_cfg
from .randomization import make_events_cfg
from .rewards import make_reward_cfg
from .termination import make_termination_cfg
from .metrics import make_metric_cfg

BUMPY_TERRAINS_CFG = TerrainGeneratorCfg(
  size=(8.0, 8.0),
  border_width=20.0,
  num_rows=10,
  num_cols=10,
  sub_terrains={
    "flat": terrain_gen.BoxFlatTerrainCfg(proportion=0.5),
    "random_rough": terrain_gen.HfRandomUniformTerrainCfg(
      proportion=0.5,
      noise_range=(0.02, 0.04),
      noise_step=0.03,
      border_width=0.25,
    ),
  },
  add_lights=True,
)

def make_scene_cfg(terrain_type: Literal["flat", "rough", "bumpy"]) -> SceneCfg:
    if terrain_type == "flat":
        terrain_cfg = TerrainEntityCfg()
    elif terrain_type == "rough":
        terrain_cfg = TerrainEntityCfg(
            terrain_type="generator",
            terrain_generator=replace(ROUGH_TERRAINS_CFG),
            max_init_terrain_level=5,
        )
        if terrain_cfg.terrain_generator:
            terrain_cfg.terrain_generator.curriculum = True
    elif terrain_type == "bumpy":
        terrain_cfg = TerrainEntityCfg(
            terrain_type="generator",
            terrain_generator=replace(BUMPY_TERRAINS_CFG),
            max_init_terrain_level=5,
        )
    else:
        raise ValueError(f"unknown terrain: {terrain_type}")

    feet_ground_cfg = ContactSensorCfg(
        name="feet_ground_contact",
        primary=ContactMatch(
            mode="subtree",
            pattern=("Right_Ankle_Cross", "Left_Ankle_Cross"),
            entity="robot",
        ),
        secondary=ContactMatch(
            mode="body",
            pattern="terrain",
        ),
        fields=("found", "force"),
        reduce="netforce",
        num_slots=1,
        track_air_time=True,
    )
    self_collision_cfg = ContactSensorCfg(
        name="self_collision",
        primary=ContactMatch(mode="subtree", pattern="Trunk", entity="robot"),
        secondary=ContactMatch(mode="subtree", pattern="Trunk", entity="robot"),
        fields=("found",),
        reduce="none",
        num_slots=1,
    )

    return SceneCfg(
        terrain=terrain_cfg,
        sensors=(feet_ground_cfg, self_collision_cfg),
        entities={"robot": get_k1_robot_cfg()},
        num_envs=1,
        extent=2.0,
    )


def make_velocity_env_cfg(play: bool) -> ManagerBasedRlEnvCfg:
    if play:
        terrain_type = "flat"
    else:
        terrain_type = "flat" # for this training flat terrain
    return ManagerBasedRlEnvCfg(
        scene=make_scene_cfg(terrain_type),
        observations=make_observation_cfg(),
        actions=make_actions_cfg(),
        commands=make_commands_cfg(),
        events=make_events_cfg(),
        rewards=make_reward_cfg(),
        terminations=make_termination_cfg(),
        curriculum=make_curriculum_cfg(terrain_type),
        metrics=make_metric_cfg(),
        viewer=ViewerConfig(
            origin_type=ViewerConfig.OriginType.ASSET_BODY,
            entity_name="robot",
            body_name="Trunk",
            distance=3.0,
            elevation=-5.0,
            azimuth=90.0,
        ),
        sim=SimulationCfg(
            nconmax=128,
            njmax=256,
            mujoco=MujocoCfg(
                timestep=0.002,
                iterations=50,
                ls_iterations=50,
            ),
        ),
        decimation=5,
        episode_length_s=30.0,
    )
