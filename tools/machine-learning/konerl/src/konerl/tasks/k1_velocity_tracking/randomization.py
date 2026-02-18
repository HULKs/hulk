from mjlab.managers.event_manager import EventTermCfg
from mjlab.managers.scene_entity_config import SceneEntityCfg
from mjlab.tasks.velocity import mdp


def make_events_cfg() -> dict[str, EventTermCfg]:
    return {
        "reset_base": EventTermCfg(
            func=mdp.reset_root_state_uniform,
            mode="reset",
            params={
                "pose_range": {"x": (-0.5, 0.5), "y": (-0.5, 0.5), "yaw": (-3.14, 3.14)},
                "velocity_range": {},
            },
        ),
        "reset_robot_joints": EventTermCfg(
            func=mdp.reset_joints_by_offset,
            mode="reset",
            params={
                "position_range": (-0.5, 0.5),
                "velocity_range": (-3.0, 3.0),
                "asset_cfg": SceneEntityCfg("robot", joint_names=(".*",)),
            },
        ),
        "push_robot": EventTermCfg(
            func=mdp.push_by_setting_velocity,
            mode="interval",
            interval_range_s=(2.0, 5.0),
            params={
                "velocity_range": {
                    "x": (-0.3, 0.3),
                    "y": (-0.3, 0.3),
                    "z": (-0.2, 0.2),
                    "roll": (-0.32, 0.32),
                    "pitch": (-0.32, 0.32),
                    "yaw": (-0.58, 0.58),
                },
            },
        ),
        "foot_friction": EventTermCfg(
            mode="startup",
            func=mdp.randomize_field,
            domain_randomization=True,
            params={
                "asset_cfg": SceneEntityCfg(
                    "robot", geom_names=("left_foot_link_collision0", "right_foot_link_collision0")
                ),
                "operation": "abs",
                "field": "geom_friction",
                "ranges": (0.3, 1.2),
            },
        ),
        "encoder_bias": EventTermCfg(
            mode="startup",
            func=mdp.randomize_encoder_bias,
            params={
                "asset_cfg": SceneEntityCfg("robot"),
                "bias_range": (-0.015, 0.015),
            },
        ),
        "base_com": EventTermCfg(
            mode="startup",
            func=mdp.randomize_field,
            domain_randomization=True,
            params={
                "asset_cfg": SceneEntityCfg("robot", body_names=("Trunk",)),  # Set per-robot.
                "operation": "add",
                "field": "body_ipos",
                "ranges": {
                    0: (-0.025, 0.025),
                    1: (-0.025, 0.025),
                    2: (-0.03, 0.03),
                },
            },
        ),
        "kp_kd": EventTermCfg(
            mode="reset",
            func=mdp.randomize_pd_gains,
            params={
                "kp_range": (0.75, 1.25),
                "kd_range": (0.75, 1.25),
            },
        ),
        "calibration_error": EventTermCfg(
            mode="reset",
            func=mdp.randomize_encoder_bias,
            params={
                "bias_range": (-0.1, 0.1),
            },
        ),
        "actuator_lag": EventTermCfg(
            mode="interval",
            interval_range_s=(0.1, 0.5),
            func=mdp.sync_actuator_delays,
            params={
                "lag_range": (-15, 15),
            },
        ),
        "effort_limits": EventTermCfg(
            mode="reset",
            func=mdp.randomize_effort_limits,
            params={
                "effort_limit_range": (0.90, 1.10),
            },
        ),
    }
