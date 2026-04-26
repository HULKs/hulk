import inspect
import torch
from mjlab.entity import Entity
from mjlab.envs import ManagerBasedRlEnv
from mjlab.managers.reward_manager import RewardTermCfg
from mjlab.managers.scene_entity_config import SceneEntityCfg
from mjlab.tasks.velocity import mdp

class BoundedPenaltyWrapper:
    """Wraps both stateless functions and stateful classes to bound their penalties."""

    def __init__(self, cfg: RewardTermCfg, env: ManagerBasedRlEnv):
        self.std = cfg.params.get("std", cfg.params.get("sigma", 1.0))
        inner_func = cfg.params["func"]
        if inspect.isclass(inner_func):
            self.inner_callable = inner_func(cfg, env)
        else:
            self.inner_callable = inner_func

    def __call__(self, env: ManagerBasedRlEnv, **kwargs) -> torch.Tensor:
        inner_kwargs = {k: v for k, v in kwargs.items() if k not in ["func", "sigma", "std"]}

        raw_penalty = self.inner_callable(env, **inner_kwargs)
        return torch.exp(-torch.abs(raw_penalty) / self.std)


def bad_base_height(
    env: ManagerBasedRlEnv,
    limit_height: float = 0.3,
    asset_cfg: SceneEntityCfg = SceneEntityCfg("robot"),
):
    """Penalizes when the base height falls below the limit height."""
    asset: Entity = env.scene[asset_cfg.name]
    return asset.data.root_link_pos_w[:, 2] < limit_height

def target_base_height(
    env: ManagerBasedRlEnv,
    target_height: float = 0.5,
    asset_cfg: SceneEntityCfg = SceneEntityCfg("robot"),
):
    """Rewards when the base height is close to the target height."""
    asset: Entity = env.scene[asset_cfg.name]
    height_error = torch.abs(asset.data.root_link_pos_w[:, 2] - target_height)
    return torch.exp(-height_error / 0.1)

def make_reward_cfg() -> dict[str, RewardTermCfg]:
    return {
        "survival": RewardTermCfg(
            func=mdp.is_alive,
            weight=2.0,
        ),
        "bad_base_height": RewardTermCfg(
            func=bad_base_height,
            weight=-20.0,
            params={"limit_height": 0.3},
        ),
        "target_base_height": RewardTermCfg(
            func=target_base_height,
            weight=1.0,
            params={"target_height": 0.5},
        ),
        "dof_pos_limits": RewardTermCfg(func=mdp.joint_pos_limits, weight=-1.0),
        "action_rate_l2": RewardTermCfg(func=mdp.action_rate_l2, weight=-0.03),
        "action_acc_l2": RewardTermCfg(func=mdp.action_acc_l2, weight=-0.02),
        "joint_vel_l2": RewardTermCfg(func=mdp.joint_vel_l2, weight=-0.005),
        "torque_l2": RewardTermCfg(func=mdp.joint_torques_l2, weight=-0.0002),
    }
