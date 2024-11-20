import mujoco
from mujoco import viewer
from numpy._typing import NDArray
import numpy as np
from nao import Nao
from transforms import (
    translation_from_isometry,
    quaternion_from_isometry,
    isometry_from_translation,
    inverse,
    Pose2,
)
from walking_types import (
    Parameters,
    Measurements,
    Control,
    Feet,
    State,
    Side,
)
from walking import step
from ground_provider import get_ground_to_robot
from forward_kinematics import RobotLegKinematics
from inverse_kinematics import LegJoints, leg_angles, foot_to_isometry
import time
from robot_dimensions import ANKLE_TO_SOLE


def default_parameters() -> Parameters:
    return Parameters(
        sole_pressure_threshold=0.5,
        min_step_duration=0.25,
        step_duration=0.25,
        foot_lift_apex=0.015,
        foot_offset_left=0.052,
        foot_offset_right=-0.052,
        base_height=0.23,
    )


def initial_state() -> State:
    return State(
        t=1.0,
        support_side=Side.LEFT,
        start_feet=Feet(
            support_sole=Pose2(),
            swing_sole=Pose2(),
        ),
        end_feet=Feet(
            support_sole=Pose2(),
            swing_sole=Pose2(),
        ),
    )


def initial_measurements() -> Measurements:
    return Measurements(
        pressure_left=0.0,
        pressure_right=1.0,
    )


def initial_control() -> Control:
    return Control(
        forward=0.06,
        turn=0.0,
        left=0.0,
    )


def get_leg_joints(nao: Nao) -> tuple[LegJoints, LegJoints]:
    left_leg_joints = LegJoints(
        hip_yaw_pitch=nao.positions.left_leg.hip_yaw_pitch.item(),
        hip_roll=nao.positions.left_leg.hip_roll.item(),
        hip_pitch=nao.positions.left_leg.hip_pitch.item(),
        knee_pitch=nao.positions.left_leg.knee_pitch.item(),
        ankle_pitch=nao.positions.left_leg.ankle_pitch.item(),
        ankle_roll=nao.positions.left_leg.ankle_roll.item(),
    )
    right_leg_joints = LegJoints(
        hip_yaw_pitch=nao.positions.left_leg.hip_yaw_pitch.item(),
        hip_roll=nao.positions.right_leg.hip_roll.item(),
        hip_pitch=nao.positions.right_leg.hip_pitch.item(),
        knee_pitch=nao.positions.right_leg.knee_pitch.item(),
        ankle_pitch=nao.positions.right_leg.ankle_pitch.item(),
        ankle_roll=nao.positions.right_leg.ankle_roll.item(),
    )
    return left_leg_joints, right_leg_joints


def apply_walking(
    nao: Nao,
    parameters: Parameters,
    state: State,
    measurements: Measurements,
    control: Control,
    dt: float,
):
    state, left_sole, left_lift, right_sole, right_lift = step(
        state,
        measurements,
        control,
        dt,
        parameters,
    )

    if left_lift < 0.001:
        measurements.pressure_left = 1.0
    else:
        measurements.pressure_left = 0.0

    if right_lift < 0.001:
        measurements.pressure_right = 1.0
    else:
        measurements.pressure_right = 0.0

    apply_inverse_kinematics(
        nao, left_sole, right_sole, left_lift, right_lift
    )


def apply_inverse_kinematics(
    nao: Nao,
    left_sole: Pose2,
    right_sole: Pose2,
    left_lift: float,
    right_lift: float,
):
    left_foot_in_walk = isometry_from_translation(-ANKLE_TO_SOLE) @ foot_to_isometry(left_sole, left_lift)
    right_foot_in_walk = isometry_from_translation(-ANKLE_TO_SOLE) @ foot_to_isometry(right_sole, right_lift)

    robot_to_origin = isometry_from_translation(nao.model.site("Robot").pos)
    walk_to_robot = isometry_from_translation(np.array([-0.02, 0.0, -0.23]))
    left_foot_in_robot = walk_to_robot @ left_foot_in_walk
    right_foot_in_robot = (
        walk_to_robot @ right_foot_in_walk
    )

    nao.model.site("LeftSole").pos = translation_from_isometry(
        robot_to_origin @ left_foot_in_robot
    )
    nao.model.site("RightSole").pos = translation_from_isometry(
        robot_to_origin @ right_foot_in_robot
    )

    left_leg_joints, right_leg_joints = leg_angles(
        walk_to_robot @ left_foot_in_walk,
        walk_to_robot @ right_foot_in_walk,
    )

    nao.actuators.left_leg.ankle_pitch = left_leg_joints.ankle_pitch
    nao.actuators.left_leg.ankle_roll = left_leg_joints.ankle_roll
    nao.actuators.left_leg.knee_pitch = left_leg_joints.knee_pitch
    nao.actuators.left_leg.hip_pitch = left_leg_joints.hip_pitch
    nao.actuators.left_leg.hip_roll = left_leg_joints.hip_roll
    nao.actuators.left_leg.hip_yaw_pitch = left_leg_joints.hip_yaw_pitch

    nao.actuators.right_leg.ankle_pitch = right_leg_joints.ankle_pitch
    nao.actuators.right_leg.ankle_roll = right_leg_joints.ankle_roll
    nao.actuators.right_leg.knee_pitch = right_leg_joints.knee_pitch
    nao.actuators.right_leg.hip_pitch = right_leg_joints.hip_pitch
    nao.actuators.right_leg.hip_roll = right_leg_joints.hip_roll


def main():
    model = mujoco.MjModel.from_xml_path("model/scene.xml")
    data = mujoco.MjData(model)
    mujoco.mj_resetDataKeyframe(model, data, 1)

    parameter = default_parameters()
    state = initial_state()
    measurements = initial_measurements()
    control = initial_control()

    nao = Nao(model, data)

    handle = viewer.launch_passive(model, data)

    dt = model.opt.timestep

    while handle.is_running():
        with handle.lock():
            start_time = time.time()
            apply_walking(nao, parameter, state, measurements, control, dt)

            mujoco.mj_step(model, data)
            end_time = time.time()
            wait_time = max(0, dt - (end_time - start_time))
        handle.sync()
        time.sleep(wait_time)


if __name__ == "__main__":
    main()
