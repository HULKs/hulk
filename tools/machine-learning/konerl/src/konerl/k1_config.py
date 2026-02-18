from pathlib import Path

import mujoco
from mjlab.actuator import BuiltinPositionActuatorCfg
from mjlab.entity import EntityArticulationInfoCfg, EntityCfg
from mjlab.utils.os import update_assets
from mjlab.utils.spec_config import CollisionCfg

K1_XML = Path("model/K1.xml").resolve()
assert K1_XML.exists()


def get_assets(meshdir: str) -> dict[str, bytes]:
    assets: dict[str, bytes] = {}
    update_assets(assets, K1_XML.parent / meshdir, meshdir)
    return assets


def get_spec() -> mujoco.MjSpec:
    spec = mujoco.MjSpec.from_file(str(K1_XML))
    spec.assets = get_assets(spec.meshdir)
    return spec


# Initial States

ZERO_POSE = EntityCfg.InitialStateCfg(
    pos=(0, 0, 0.6),
    joint_pos={"ALeft_Shoulder_Pitch": 0.25, 
               "ARight_Shoulder_Pitch": 0.25, 
               "Right_Shoulder_Roll": 1.4, 
               "Left_Shoulder_Roll": -1.4, 
               "Left_Elbow_Pitch": 0.15,
               "Right_Elbow_Pitch": 0.15,
               "Left_Elbow_Yaw": -2.25, 
               "Right_Elbow_Yaw": 2.25,
               "Left_Hip_Pitch": -0.2, 
               "Right_Hip_Pitch": -0.2,
               "Left_Hip_Roll": 0.1, 
               "Right_Hip_Roll": -0.1,
               "Left_Knee_Pitch": 0.4,
               "Right_Knee_Pitch": 0.4,
               "Left_Ankle_Pitch": -0.2,
               "Right_Ankle_Pitch": -0.2,
               "Left_Ankle_Roll": -0.1,
               "Right_Ankle_Roll": 0.1,
               ".*": 0.0},
    joint_vel={".*": 0.0},
)

# Collison Configs

FULL_COLLISION = CollisionCfg(
    geom_names_expr=(r".*_collision.*",),
    condim={r"(left|right)_foot_link_collision0": 3, ".*_collision": 1},
    priority={r"(left|right)_foot_link_collision0": 1},
    friction={r"(left|right)_foot_link_collision0": (0.6,)},
)

FEET_ONLY_COLLISION = CollisionCfg(
    geom_names_expr=("left_foot_link_collision0", "right_foot_link_collision0"),
    contype=1,
    conaffinity=1,
    condim=3,
    priority=1,
    friction=(0.6,),
    solimp=(0.9, 0.95, 0.01),
    solref=(0.02, 1),
)

# Actuators and Articulation

HEAD = BuiltinPositionActuatorCfg(
    target_names_expr=("AAHead_yaw", "Head_pitch"),
    stiffness=10.0,
    damping=1.0,
    effort_limit=6.0,
)

SHOULDER_PITCH = BuiltinPositionActuatorCfg(
    target_names_expr=("ALeft_Shoulder_Pitch", "ARight_Shoulder_Pitch"),
    stiffness=10.0,
    damping=0.1,
    effort_limit=14.0,
)

SHOULDER_ROLL = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Shoulder_Roll", "Right_Shoulder_Roll"),
    stiffness=10.0,
    damping=0.1,
    effort_limit=14.0,
)

ELBOW_PITCH = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Elbow_Pitch", "Right_Elbow_Pitch"),
    stiffness=10.0,
    damping=0.1,
    effort_limit=14.0,
)

ELBOW_YAW = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Elbow_Yaw", "Right_Elbow_Yaw"),
    stiffness=10.0,
    damping=0.1,
    effort_limit=14.0,
)

HIP_PITCH = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Hip_Pitch", "Right_Hip_Pitch"),
    stiffness=40.0,
    damping=2.0,
    effort_limit=30.0,
)

HIP_ROLL = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Hip_Roll", "Right_Hip_Roll"),
    stiffness=40.0,
    damping=2.0,
    effort_limit=35.0,
)

HIP_YAW = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Hip_Yaw", "Right_Hip_Yaw"),
    stiffness=40.0,
    damping=2.0,
    effort_limit=20.0,
)

KNEE_PITCH = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Knee_Pitch", "Right_Knee_Pitch"),
    stiffness=40.0,
    damping=2.0,
    effort_limit=40.0,
)

ANKLE_PITCH = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Ankle_Pitch", "Right_Ankle_Pitch"),
    stiffness=35.0,
    damping=0.2,
    effort_limit=15.0,
)

ANKLE_ROLL = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Ankle_Roll", "Right_Ankle_Roll"),
    stiffness=35.0,
    damping=0.2,
    effort_limit=15.0,
)

K1_ARTICULATION = EntityArticulationInfoCfg(
    actuators=(
        # HEAD,
        SHOULDER_PITCH,
        SHOULDER_ROLL,
        ELBOW_PITCH,
        ELBOW_YAW,
        HIP_PITCH,
        HIP_ROLL,
        HIP_YAW,
        KNEE_PITCH,
        ANKLE_PITCH,
        ANKLE_ROLL,
    ),
    soft_joint_pos_limit_factor=0.9,
)


def get_k1_robot_cfg() -> EntityCfg:
    return EntityCfg(
        init_state=ZERO_POSE,
        collisions=(FEET_ONLY_COLLISION,),
        spec_fn=get_spec,
        articulation=K1_ARTICULATION,
    )


if __name__ == "__main__":
    import mujoco.viewer as viewer
    from mjlab.scene import Scene, SceneCfg
    from mjlab.terrains import TerrainImporterCfg

    scene = Scene(SceneCfg(terrain=TerrainImporterCfg(), entities={"robot": get_k1_robot_cfg()}), "cpu")
    model = scene.compile()
    # model.opt.gravity[:] = 0.0
    viewer.launch(model)
