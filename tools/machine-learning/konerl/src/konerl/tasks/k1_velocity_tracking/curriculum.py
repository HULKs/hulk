from typing import Literal

from typing import TypedDict

import torch

from mjlab.managers.curriculum_manager import CurriculumTermCfg
from mjlab.envs import ManagerBasedRlEnv
from mjlab.tasks.velocity import mdp

STAGES = [0, 500 * 24, 1000 * 24, 2000 * 24, 3000 * 24, 4000 * 24]

def make_curriculum_cfg(terrain_type: Literal["flat", "rough", "bumpy"]) -> dict[str, CurriculumTermCfg]:
    curriculum = {
        # Todo: add "lin_vel_y": (?, ?)
        "command_vel": CurriculumTermCfg(
            func=mdp.commands_vel,
            params={
                "command_name": "twist",
                "velocity_stages": [
                    {"step": STAGES[0], "lin_vel_x": (-0.5, 0.8), "ang_vel_z": (-0.2, 0.2)},
                    {"step": STAGES[1], "lin_vel_x": (-0.8, 1.2), "ang_vel_z": (-0.5, 0.5)},
                    {"step": STAGES[2], "lin_vel_x": (-1.2, 1.6), "ang_vel_z": (-0.8, 0.8)},
                    {"step": STAGES[3], "lin_vel_x": (-1.5, 2.0), "ang_vel_z": (-1.2, 1.2)},
                    {"step": STAGES[4], "lin_vel_x": (-1.5, 2.0), "ang_vel_z": (-1.7, 1.7)},
                ],
            },
        ),
        "upright_weight": CurriculumTermCfg(
            func=mdp.reward_weight,
            params={
                "reward_name": "upright",
                "weight_stages": [
                    {"step": STAGES[0], "weight": 1},
                    {"step": STAGES[1], "weight": 0.8},
                    {"step": STAGES[2], "weight": 0.6},
                    {"step": STAGES[3], "weight": 0.4},
                    {"step": STAGES[4], "weight": 0.3},
                ],
            },
        ),
        "joint_torques_weight": CurriculumTermCfg(
            func=mdp.reward_weight,
            params={
                "reward_name": "joint_torques",
                "weight_stages": [
                    {"step": STAGES[0], "weight": -1e-6},
                    {"step": STAGES[1], "weight": -5e-6},
                    {"step": STAGES[2], "weight": -1e-5},
                    {"step": STAGES[3], "weight": -5e-5},
                ],
            },
        ),
        "left_foot_flat_orientation_weight": CurriculumTermCfg(
            func=mdp.reward_weight,
            params={
                "reward_name": "left_foot_flat_orientation",
                "weight_stages": [
                    {"step": STAGES[0], "weight": 1.0},
                    {"step": STAGES[1], "weight": 0.8},
                    {"step": STAGES[2], "weight": 0.6},
                ],
            },
        ),
        "right_foot_flat_orientation_weight": CurriculumTermCfg(
            func=mdp.reward_weight,
            params={
                "reward_name": "right_foot_flat_orientation",
                "weight_stages": [
                    {"step": STAGES[0], "weight": 1.0},
                    {"step": STAGES[1], "weight": 0.8},
                    {"step": STAGES[2], "weight": 0.6},
                ],
            },
        ),
        "soft_landing_weight": CurriculumTermCfg(
            func=mdp.reward_weight,
            params={
                "reward_name": "soft_landing",
                "weight_stages": [
                    {"step": STAGES[0], "weight": -5e-5},
                    {"step": STAGES[2], "weight": -1e-4},
                    {"step": STAGES[4], "weight": -5e-4},
                ]
            }
        )
    }
    if terrain_type == "rough":
        curriculum["terrain_levels"] = CurriculumTermCfg(
            func=mdp.terrain_levels_vel,
            params={"command_name": "twist"},
        )

    return curriculum

class RandomizerParameterStage(TypedDict):
  step: int
  param_name: str
  param_value: tuple[float, float]

def randomization_params(
  env: ManagerBasedRlEnv,
  env_ids: torch.Tensor,
  randomizer_name: str,
  parameter_stages: list[RandomizerParameterStage],
) -> tuple[float, float]:
  """Update a randomizers parameters based on training step stages."""
  del env_ids
  random_term_cfg = env.event_manager.get_term_cfg(randomizer_name)
  for stage in parameter_stages:
    if env.common_step_counter > stage["step"]:
      random_term_cfg.params[stage["param_name"]] = stage["param_value"]
  return random_term_cfg.params[parameter_stages[0]["param_name"]]