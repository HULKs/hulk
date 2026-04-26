from mjlab.managers.event_manager import EventTermCfg
from mjlab.managers.scene_entity_config import SceneEntityCfg
from mjlab.envs.mdp import events as event_fns, dr

def make_events_cfg() -> dict[str, EventTermCfg]:
    events = {
        "reset_scene": EventTermCfg(
            func=event_fns.reset_scene_to_default,
            mode="reset",
        ),
        "reset_base": EventTermCfg(
            func=event_fns.reset_root_state_uniform,
            mode="reset",
            params={
                "pose_range": {"x": (-0.5, 0.5), "y": (-0.5, 0.5), "yaw": (-3.14, 3.14)},
                "velocity_range": {},
            },
        ),
        "reset_robot_joints": EventTermCfg(
            func=event_fns.reset_joints_by_offset,
            mode="reset",
            params={
                "position_range": (-0.2, 0.2),
                "velocity_range": (-0.2, 0.2),
                "asset_cfg": SceneEntityCfg("robot", joint_names=(".*",)),
            },
        ),
        "push_robot": EventTermCfg(
            func=event_fns.apply_body_impulse,
            mode="step",
            min_step_count_between_reset=500,
            params={
                "force_range": (-150.0, 150.0),
                "torque_range": (-10.0, 10.0),
                "duration_s": (0.01, 0.2),
                "cooldown_s": (10.0, 15.0),
                "asset_cfg": SceneEntityCfg("robot", body_names=("Trunk",)),
            },
        ),
        "impulse": EventTermCfg(
            func=event_fns.apply_body_impulse,
            mode="step",
            min_step_count_between_reset=500,
            params={
                "force_range": (-20.0, 20.0),
                "torque_range": (-5.0, 5.0),
                "duration_s": (0.3, 2.0),
                "cooldown_s": (10.0, 15.0),
                "asset_cfg": SceneEntityCfg("robot", body_names=("Trunk",)),
            },
        ),
        "foot_friction": EventTermCfg(
            func=dr.geom_friction,
            mode="reset",
            params={
                "asset_cfg": SceneEntityCfg("robot", geom_names=(r"(left|right)_foot", )),
                "ranges": (1.6, 2.4),
                "operation": "abs",
                "shared_random": True,
            },
        ),
        "terrain_friction": EventTermCfg(
            func=dr.geom_friction,
            mode="reset",
            params={
                "asset_cfg": SceneEntityCfg("terrain"), 
                "ranges": (1.6, 2.4),
                "operation": "abs",
                "shared_random": True,
            },
        ),
        "encoder_bias": EventTermCfg(
            mode="reset",
            func=dr.encoder_bias,
            params={
                "asset_cfg": SceneEntityCfg("robot"),
                "bias_range": (-0.015, 0.015),
            },
        ),
        "base_com": EventTermCfg(
            mode="reset",
            func=dr.body_com_offset,
            params={
                "asset_cfg": SceneEntityCfg("robot", body_names=("Trunk",)),  # Set per-robot.
                "operation": "add",
                "ranges": {
                    0: (-0.03, 0.03),
                    1: (-0.03, 0.03),
                    2: (-0.03, 0.03),
                },
            },
        ),
        "trunk_mass": EventTermCfg(
            mode="reset",
            func=dr.body_mass,
            params={
                "asset_cfg": SceneEntityCfg("robot", body_names=("Trunk",)),  # Set per-robot.
                "operation": "scale",
                "ranges": (0.85, 1.15),
            },
        ),
        "default_kp_kd": EventTermCfg(
            mode="reset",
            func=dr.pd_gains,
            params={
                "asset_cfg": SceneEntityCfg("robot", joint_names=(
                    ".*Hip_Pitch.*",
                    ".*Hip_Yaw.*",
                    ".*Knee_Pitch.*",
                    )),
                "kp_range": (0.95, 1.05),
                "kd_range": (0.95, 1.05),
            },
        ),
        "special_kp_kd": EventTermCfg(
            mode="reset",
            func=dr.pd_gains,
            params={
                "asset_cfg": SceneEntityCfg("robot", joint_names=(
                    ".*Hip_Roll.*",
                    ".*Ankle_Pitch.*",
                    ".*Ankle_Roll.*"
                    )),
                "kp_range": (0.8, 1.2),
                "kd_range": (0.8, 1.2),
            },
        ),
        "armature": EventTermCfg(
            mode="reset",
            func=dr.joint_armature,
            params={
                "asset_cfg": SceneEntityCfg("robot", joint_names=(".*",)),
                "ranges": (0.8, 1.2),
                "operation": "scale",
            },   
        ),
    }

    delay_randomizer = getattr(dr, "sync_actuator_delays", None) or getattr(dr, "actuator_delays", None)
    if delay_randomizer is not None:
        events["actuator_lag"] = EventTermCfg(
            mode="interval",
            interval_range_s=(0.01, 0.01),
            func=delay_randomizer,
            params={
                "lag_range": (1, 3),
            },
        )

    return events
