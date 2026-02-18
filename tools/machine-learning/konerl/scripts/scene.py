from mjlab.envs.mdp import SceneEntityCfg
from mjlab.scene import Scene
from mujoco import MjData

from konerl.tasks.k1_velocity_tracking.env_cfg import k1_rough_env_cfg

cfg = k1_rough_env_cfg()
scene = Scene(cfg.scene, "cpu")
model = scene.compile()
data = MjData(model)


cfg = SceneEntityCfg("robot")
cfg.resolve(scene)

breakpoint()
