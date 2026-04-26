from pathlib import Path

import mujoco
from mjlab.actuator.builtin_actuator import BuiltinPositionActuatorCfg as BuiltinPositionActuatorCfg
from mjlab.entity import EntityArticulationInfoCfg, EntityCfg
from mjlab.utils.spec_config import CollisionCfg

K1_XML = Path("model/K1.xml").resolve()
assert K1_XML.exists()


def get_assets(meshdir: str) -> dict[str, bytes]:
    assets: dict[str, bytes] = {}
    mesh_root = (K1_XML.parent / meshdir).resolve()
    if not mesh_root.exists():
        return assets
    for path in mesh_root.rglob("*"):
        if not path.is_file():
            continue
        rel = path.relative_to(mesh_root).as_posix()
        assets[f"{meshdir}/{rel}"] = path.read_bytes()
    return assets


def get_spec() -> mujoco.MjSpec:
    spec = mujoco.MjSpec.from_file(str(K1_XML))
    for geom in tuple(spec.geoms):
        if geom.name == "ground":
            spec.delete(geom)  # type: ignore[attr-defined]
            break
    spec.assets = get_assets(spec.meshdir)
    return spec


# Initial States

ZERO_POSE = EntityCfg.InitialStateCfg(
    pos=(0, 0, 0.6),
    joint_pos={"ALeft_Shoulder_Pitch": 0.25, 
               "Left_Shoulder_Roll": -1.4, 
               "Left_Elbow_Pitch": 0.15,
               "Left_Elbow_Yaw": -2.25, 

               "ARight_Shoulder_Pitch": 0.25, 
               "Right_Shoulder_Roll": 1.4, 
               "Right_Elbow_Pitch": 0.15,
               "Right_Elbow_Yaw": 2.25,

               "Left_Hip_Pitch": -0.3, 
               "Left_Hip_Roll": 0.1, 
               "Left_Knee_Pitch": 0.6,
               "Left_Ankle_Pitch": -0.3,
               "Left_Ankle_Roll": -0.1,

               "Right_Hip_Pitch": -0.3,
               "Right_Hip_Roll": -0.1,
               "Right_Knee_Pitch": 0.6,
               "Right_Ankle_Pitch": -0.3,
               "Right_Ankle_Roll": 0.1,
               ".*": 0.0},
    joint_vel={".*": 0.0},
)

# Collison Configs

FULL_COLLISION = CollisionCfg(
    geom_names_expr=(r".*_collision.*", r"(left|right)_foot"),
    condim=3,
    priority=1,
    friction=(2.0,),
    solimp=(0.01, 0.995, 0.0),
    solref=(0.01, 1.0),
)

FEET_ONLY_COLLISION = CollisionCfg(
    geom_names_expr=(r"(left|right)_foot",),
    contype=1,
    conaffinity=1,
    condim=3,
    priority=1,
    friction=(2.0,),
    solimp=(0.01, 0.995, 0.025),
    solref=(0.01, 1.0),
)

# Actuators and Articulation

HEAD = BuiltinPositionActuatorCfg(
    target_names_expr=("AAHead_yaw", "Head_pitch"),
    stiffness=10.0,
    damping=1.0,
    effort_limit=6.0,
    delay_min_lag=1,
    delay_max_lag=2,
)

SHOULDER_PITCH = BuiltinPositionActuatorCfg(
    target_names_expr=("ALeft_Shoulder_Pitch", "ARight_Shoulder_Pitch"),
    stiffness=10.0,
    damping=1.0,
    effort_limit=14.0,
    delay_min_lag=1,
    delay_max_lag=2,
)

SHOULDER_ROLL = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Shoulder_Roll", "Right_Shoulder_Roll"),
    stiffness=10.0,
    damping=1.0,
    effort_limit=14.0,
    delay_min_lag=1,
    delay_max_lag=2,
)

ELBOW_PITCH = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Elbow_Pitch", "Right_Elbow_Pitch"),
    stiffness=10.0,
    damping=1.0,
    effort_limit=14.0,
    delay_min_lag=1,
    delay_max_lag=2,
)

ELBOW_YAW = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Elbow_Yaw", "Right_Elbow_Yaw"),
    stiffness=10.0,
    damping=1.0,
    effort_limit=14.0,
    delay_min_lag=1,
    delay_max_lag=2,
)

HIP_PITCH = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Hip_Pitch", "Right_Hip_Pitch"),
    stiffness=80.0,
    damping=4.0,
    effort_limit=30.0,
    delay_min_lag=1,
    delay_max_lag=2,
)

HIP_ROLL = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Hip_Roll", "Right_Hip_Roll"),
    stiffness=80.0,
    damping=4.0,
    effort_limit=35.0,
    delay_min_lag=1,
    delay_max_lag=2,
)

HIP_YAW = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Hip_Yaw", "Right_Hip_Yaw"),
    stiffness=80.0,
    damping=4.0,
    effort_limit=20.0,
    delay_min_lag=1,
    delay_max_lag=2,
)

KNEE_PITCH = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Knee_Pitch", "Right_Knee_Pitch"),
    stiffness=80.0,
    damping=4.0,
    effort_limit=40.0,
    delay_min_lag=1,
    delay_max_lag=2,
)

ANKLE_PITCH = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Ankle_Pitch", "Right_Ankle_Pitch"),
    stiffness=25.0,
    damping=2.0,
    effort_limit=20.0,
    delay_min_lag=1,
    delay_max_lag=2,
)

ANKLE_ROLL = BuiltinPositionActuatorCfg(
    target_names_expr=("Left_Ankle_Roll", "Right_Ankle_Roll"),
    stiffness=60.0,
    damping=2.0,
    effort_limit=20.0,
    delay_min_lag=1,
    delay_max_lag=2,
)

K1_ARTICULATION = EntityArticulationInfoCfg(
    actuators=(
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
        collisions=(FULL_COLLISION,),
        spec_fn=get_spec,
        articulation=K1_ARTICULATION,
    )


if __name__ == "__main__":
    import mujoco.viewer as viewer
    from mjlab.scene import Scene, SceneCfg
    from mjlab.terrains import TerrainEntityCfg

    scene = Scene(SceneCfg(terrain=TerrainEntityCfg(), entities={"robot": get_k1_robot_cfg()}), "cpu")
    model = scene.compile()
    viewer.launch(model)
