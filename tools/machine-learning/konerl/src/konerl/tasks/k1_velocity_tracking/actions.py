from mjlab.envs.mdp.actions import JointPositionActionCfg
from mjlab.managers.action_manager import ActionTermCfg
from mjlab.managers.command_manager import CommandTermCfg

from .smoothed_command import SmoothedVelocityCommandCfg

DEFAULT_MOTION_RANGES = SmoothedVelocityCommandCfg.SmoothRanges(
    lin_vel_x=(-1.0, 1.0),
    lin_vel_y=(-1.0, 1.0),
    ang_vel_z=(-1.0, 1.0),
)

REL_STANDING_ENVS = 1.0 # Just standing...

def make_actions_cfg() -> dict[str, ActionTermCfg]:
    return {
        "joint_pos": JointPositionActionCfg(
            entity_name="robot",
            actuator_names=(".*",),
            scale=0.5,  # Override per-robot.
            use_default_offset=True,
        ),
    }


def make_commands_cfg() -> dict[str, CommandTermCfg]:
    ranges =  DEFAULT_MOTION_RANGES

    command = SmoothedVelocityCommandCfg(
        entity_name="robot",
        ema_alpha=0.04,
        resampling_time_range=(3.0, 8.0),
        rel_standing_envs=REL_STANDING_ENVS,
        rel_heading_envs=0.0,
        heading_command=False,
        debug_vis=True,
        ranges=ranges,
    )
    command.viz.z_offset = 1.15
    return {"twist": command}
