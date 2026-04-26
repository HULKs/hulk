from typing import Literal

from mjlab.managers.curriculum_manager import CurriculumTermCfg
from mjlab.tasks.velocity import mdp


def make_curriculum_cfg(terrain_type: Literal["flat", "rough", "bumpy"]) -> dict[str, CurriculumTermCfg]:
    curriculum = {}
    if terrain_type == "rough":
        curriculum["terrain_levels"] = CurriculumTermCfg(
            func=mdp.terrain_levels_vel,
            params={"command_name": "twist"},
        )

    return curriculum
