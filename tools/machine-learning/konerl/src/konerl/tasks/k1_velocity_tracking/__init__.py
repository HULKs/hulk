from mjlab.tasks.registry import register_mjlab_task
from mjlab.tasks.velocity.rl import VelocityOnPolicyRunner

from .env_cfg import k1_rough_env_cfg
from .rl_cfg import k1_ppo_runner_cfg

register_mjlab_task(
    task_id="Mjlab-Velocity-Rough-K1",
    env_cfg=k1_rough_env_cfg(),
    play_env_cfg=k1_rough_env_cfg(play=True),
    rl_cfg=k1_ppo_runner_cfg(),
    runner_cls=VelocityOnPolicyRunner,
)
