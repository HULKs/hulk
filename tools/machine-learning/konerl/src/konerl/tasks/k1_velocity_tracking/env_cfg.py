from mjlab.envs import ManagerBasedRlEnvCfg
from mjlab.envs import mdp as envs_mdp
from mjlab.managers.event_manager import EventTermCfg

from .simulation import make_velocity_env_cfg


def k1_rough_env_cfg(*, play: bool = False) -> ManagerBasedRlEnvCfg:
    cfg = make_velocity_env_cfg(play)
    if play:
        cfg.episode_length_s = int(1e9)

        cfg.observations["policy"].enable_corruption = False
        _ = cfg.events.pop("push_robot", None)
        cfg.events["randomize_terrain"] = EventTermCfg(
            func=envs_mdp.randomize_terrain,
            mode="reset",
            params={},
        )
        if cfg.scene.terrain is not None:
            if cfg.scene.terrain.terrain_generator is not None:
                cfg.scene.terrain.terrain_generator.curriculum = False
                cfg.scene.terrain.terrain_generator.num_cols = 5
                cfg.scene.terrain.terrain_generator.num_rows = 5
                cfg.scene.terrain.terrain_generator.border_width = 10.0

    return cfg


if __name__ == "__main__":
    import mujoco.viewer as viewer
    from mjlab.scene import Scene
    from mujoco import MjData

    cfg = k1_rough_env_cfg()
    scene = Scene(cfg.scene, "cpu")
    model = scene.compile()
    data = MjData(model)
    data.qpos[0] = 5.0
    data.qpos[1] = 50.0
    viewer.launch(model, data)
