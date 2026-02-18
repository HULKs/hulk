import math

from mjlab.managers.reward_manager import RewardTermCfg
from mjlab.managers.scene_entity_config import SceneEntityCfg
from mjlab.tasks.velocity import mdp
from mjlab.envs import ManagerBasedRlEnv
from mjlab.sensor import ContactSensor
import torch


def foot_contact_count_consistency(
    env: ManagerBasedRlEnv,
    sensor_name: str,
    command_name: str,
    walking_threshold: float = 0.05,
    running_threshold: float = 1.0,
) -> torch.Tensor:
    """Penalize deviation from target foot contact counts for a 2-legged robot."""
    contact_sensor: ContactSensor = env.scene[sensor_name]
    command = env.command_manager.get_command(command_name)
    assert command is not None
    assert contact_sensor.data.found is not None
    linear_norm = torch.norm(command[:, :2], dim=1)
    angular_norm = torch.abs(command[:, 2])
    total_command = linear_norm + angular_norm
    in_contact = (contact_sensor.data.found > 0).float()
    current_contacts = torch.sum(in_contact, dim=1)
    target_contacts = torch.full_like(current_contacts, 1.0)
    target_contacts[total_command <= walking_threshold] = 2.0
    target_contacts[total_command > running_threshold] = 0.5
    cost = torch.square(current_contacts - target_contacts)
    num_envs = current_contacts.shape[0]
    mean_contacts = torch.sum(current_contacts) / num_envs
    env.extras["log"]["Metrics/foot_contacts_mean"] = mean_contacts

    return cost




def make_reward_cfg() -> dict[str, RewardTermCfg]:
    site_names = ("left_foot", "right_foot")
    return {
        "track_linear_velocity": RewardTermCfg(
            func=mdp.track_linear_velocity,
            weight=1.0,
            params={"command_name": "twist", "std": math.sqrt(0.2)},
        ),
        "track_angular_velocity": RewardTermCfg(
            func=mdp.track_angular_velocity,
            weight=1.0,
            params={"command_name": "twist", "std": math.sqrt(0.2)},
        ),
        "upright": RewardTermCfg(
            func=mdp.flat_orientation,
            weight=0.8,
            params={
                "std": math.sqrt(0.3),
                "asset_cfg": SceneEntityCfg("robot", body_names=("Trunk")),
            },
        ),
        "pose": RewardTermCfg(
            func=mdp.variable_posture,
            weight=1.0,
            params={
                "asset_cfg": SceneEntityCfg("robot", joint_names=(".*",)),
                "command_name": "twist",
                "std_standing": {".*": 0.02},
                "std_walking": {
                    # Lower body.
                    r".*Hip_Pitch.*": 0.3,
                    r".*Hip_Roll.*": 0.15,
                    r".*Hip_Yaw.*": 0.15,
                    r".*Knee_Pitch.*": 0.35,
                    r".*Ankle_Pitch.*": 0.25,
                    r".*Ankle_Roll.*": 0.1,
                    # Arms.
                    r".*Shoulder_Pitch.*": 0.15,
                    r".*Shoulder_Roll.*": 0.15,
                    r".*Elbow_Pitch.*": 0.15,
                    r".*Elbow_Yaw.*": 0.15,
                },
                "std_running": {
                    r".*Hip_Pitch.*": 0.5,
                    r".*Hip_Roll.*": 0.2,
                    r".*Hip_Yaw.*": 0.2,
                    r".*Knee_Pitch.*": 0.6,
                    r".*Ankle_Pitch.*": 0.35,
                    r".*Ankle_Roll.*": 0.15,
                    # Arms.
                    r".*Shoulder_Pitch.*": 0.5,
                    r".*Shoulder_Roll.*": 0.2,
                    r".*Elbow_Pitch.*": 0.35,
                    r".*Elbow_Yaw.*": 0.15,
                },
                "walking_threshold": 0.05,
                "running_threshold": 1.0,
            },
        ),
        "body_ang_vel": RewardTermCfg(
            func=mdp.body_angular_velocity_penalty,
            weight=-0.05,  # Override per-robot
            params={"asset_cfg": SceneEntityCfg("robot", body_names=("Trunk"))},
        ),
        "angular_momentum": RewardTermCfg(
            func=mdp.angular_momentum_penalty,
            weight=-0.02,  # Override per-robot
            params={"sensor_name": "robot/root_angmom"},
        ),
        "dof_pos_limits": RewardTermCfg(func=mdp.joint_pos_limits, weight=-1.0),
        "action_rate_l2": RewardTermCfg(func=mdp.action_rate_l2, weight=-0.05),
        "air_time": RewardTermCfg(
            func=mdp.feet_air_time,
            weight=0.1,  # Override per-robot.
            params={
                "sensor_name": "feet_ground_contact",
                "threshold_min": 0.04,
                "threshold_max": 0.4,
                "command_name": "twist",
                "command_threshold": 0.05,
            },
        ),
        "foot_clearance": RewardTermCfg(
            func=mdp.feet_clearance,
            weight=-0.2,
            params={
                "target_height": 0.15,
                "command_name": "twist",
                "command_threshold": 0.05,
                "asset_cfg": SceneEntityCfg("robot", site_names=site_names),  # Set per-robot.
            },
        ),
        "foot_swing_height": RewardTermCfg(
            func=mdp.feet_swing_height,
            weight=-0.25,
            params={
                "sensor_name": "feet_ground_contact",
                "target_height": 0.15,
                "command_name": "twist",
                "command_threshold": 0.05,
                "asset_cfg": SceneEntityCfg("robot", site_names=site_names),  # Set per-robot.
            },
        ),
        "foot_slip": RewardTermCfg(
            func=mdp.feet_slip,
            weight=-0.1,
            params={
                "sensor_name": "feet_ground_contact",
                "command_name": "twist",
                "command_threshold": 0.05,
                "asset_cfg": SceneEntityCfg("robot", site_names=site_names),  # Set per-robot.
            },
        ),
        "soft_landing": RewardTermCfg(
            func=mdp.soft_landing,
            weight=-5e-5,
            params={
                "sensor_name": "feet_ground_contact",
                "command_name": "twist",
                "command_threshold": 0.05,
            },
        ),
        "self_collisions": RewardTermCfg(
            func=mdp.self_collision_cost,
            weight=-1.0,
            params={"sensor_name": "self_collision"},
        ),
        "joint_torques": RewardTermCfg(
            func=mdp.joint_torques_l2,
            weight=-1e-4,
        ),
        "left_foot_flat_orientation": RewardTermCfg(
            func=mdp.flat_orientation,
            weight=0.2,
            params={
                "std": 0.05,
                "asset_cfg": SceneEntityCfg(
                   "robot", 
                    body_names=("left_foot_link")
                ),
            },
        ),
        "right_foot_flat_orientation": RewardTermCfg(
            func=mdp.flat_orientation,
            weight=0.2,
            params={
                "std": 0.05,
                "asset_cfg": SceneEntityCfg(
                   "robot", 
                    body_names=("right_foot_link")
                ),
            },
        ),
        "number_feet_on_ground": RewardTermCfg(
            func=foot_contact_count_consistency,
            weight=-0.3,
            params={
                "sensor_name": "feet_ground_contact",
                "command_name": "twist",
                "walking_threshold": 0.05,
                "running_threshold": 1.0,
            },
        )
    }
