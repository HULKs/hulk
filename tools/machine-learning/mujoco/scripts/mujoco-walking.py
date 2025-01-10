import time

import mujoco
import walking_engine
from kinematics import LegJoints
from mujoco import viewer
from nao_interface import Nao
from nao_interface.poses import PENALIZED_POSE
from throwing import ThrowableObject
from transforms import (
    Pose2,
)
from walking_engine import (
    Control,
    Feet,
    Measurements,
    Parameters,
    Side,
    State,
)


def default_parameters() -> Parameters:
    return Parameters(
        sole_pressure_threshold=5.0,
        min_step_duration=0.25,
        step_duration=0.25,
        foot_lift_apex=0.015,
        foot_offset_left=0.052,
        foot_offset_right=-0.052,
        walk_height=0.23,
    )


def initial_state() -> State:
    return State(
        t=1.0,
        support_side=Side.RIGHT,
        start_feet=Feet(
            support_sole=Pose2(),
            swing_sole=Pose2(),
        ),
        end_feet=Feet(
            support_sole=Pose2(),
            swing_sole=Pose2(),
        ),
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
    state, left_sole, left_lift, right_sole, right_lift = walking_engine.step(
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

    lower_body_joints = walking_engine.compute_lower_body_joints(
        left_sole, right_sole, left_lift, right_lift
    )

    nao.actuators.left_leg.ankle_pitch = lower_body_joints.left.ankle_pitch
    nao.actuators.left_leg.ankle_roll = lower_body_joints.left.ankle_roll
    nao.actuators.left_leg.knee_pitch = lower_body_joints.left.knee_pitch
    nao.actuators.left_leg.hip_pitch = lower_body_joints.left.hip_pitch
    nao.actuators.left_leg.hip_roll = lower_body_joints.left.hip_roll
    nao.actuators.left_leg.hip_yaw_pitch = lower_body_joints.left.hip_yaw_pitch

    nao.actuators.right_leg.ankle_pitch = lower_body_joints.right.ankle_pitch
    nao.actuators.right_leg.ankle_roll = lower_body_joints.right.ankle_roll
    nao.actuators.right_leg.knee_pitch = lower_body_joints.right.knee_pitch
    nao.actuators.right_leg.hip_pitch = lower_body_joints.right.hip_pitch
    nao.actuators.right_leg.hip_roll = lower_body_joints.right.hip_roll


def main():
    model = mujoco.MjModel.from_xml_path("model/scene.xml")
    data = mujoco.MjData(model)
    mujoco.mj_resetDataKeyframe(model, data, 1)

    parameter = default_parameters()
    state = initial_state()
    control = initial_control()

    nao = Nao(model, data)
    nao.reset(PENALIZED_POSE)

    handle = viewer.launch_passive(model, data)

    dt = model.opt.timestep
    throwable = ThrowableObject(model, data, "floor", "tomato")

    while handle.is_running():
        with handle.lock():
            start_time = time.time()
            fsr_positions = [
                "rear_left",
                "rear_right",
                "front_left",
                "front_right",
            ]
            right_pressure = sum(
                nao.data.sensor(f"force_sensitive_resistors.right.{pos}").data
                for pos in fsr_positions
            )
            left_pressure = sum(
                nao.data.sensor(f"force_sensitive_resistors.left.{pos}").data
                for pos in fsr_positions
            )
            measurements = Measurements(left_pressure, right_pressure)

            if throwable.has_ground_contact():
                target = data.joint("root").qpos[:3]
                throwable.random_throw(target, time_to_reach=0.2, distance=0.5)

            if (
                measurements.pressure_left > 0.0
                or measurements.pressure_right > 0.0
            ):
                apply_walking(nao, parameter, state, measurements, control, dt)

            mujoco.mj_step(model, data)
            end_time = time.time()
            wait_time = max(0, dt - (end_time - start_time))
        handle.sync()
        time.sleep(wait_time)


if __name__ == "__main__":
    main()
