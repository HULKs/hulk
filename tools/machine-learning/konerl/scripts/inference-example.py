import argparse
import time
from pathlib import Path

import mujoco
import mujoco.viewer
import numpy as np
import onnxruntime as ort
from mjlab.managers.observation_manager import ObservationManager
from mjlab.scene import Scene
from scipy.spatial.transform import Rotation as R

from konerl.k1_config import ZERO_POSE
from konerl.tasks.k1_velocity_tracking.env_cfg import k1_rough_env_cfg
from konerl.tasks.k1_velocity_tracking.observations import make_observation_cfg


def get_default_joint_pos(model, joint_name):
    """Resolves default position from the ZERO_POSE config using regex logic."""
    if joint_name in ZERO_POSE.joint_pos:
        return ZERO_POSE.joint_pos[joint_name]

    for key, val in ZERO_POSE.joint_pos.items():
        if key == ".*":
            continue
        clean_key = key.replace(".*", "")
        if clean_key in joint_name:
            return val

    return ZERO_POSE.joint_pos.get(".*", 0.0)


def main(onnx_model_path: Path):
    print("Building K1 environment...")

    cfg = k1_rough_env_cfg(play=True)
    scene = Scene(cfg.scene, "cpu")
    model = scene.compile()
    data = mujoco.MjData(model)

    print(f"Loading policy from: {onnx_model_path}")
    ort_sess = ort.InferenceSession(onnx_model_path)
    input_name = ort_sess.get_inputs()[0].name

    actuator_joint_ids = []
    default_dof_pos = []

    print(f"Robot has {model.nu} actuators.")

    for i in range(model.nu):
        jnt_id = model.actuator_trnid[i, 0]
        actuator_joint_ids.append(jnt_id)

        jnt_name = mujoco.mj_id2name(model, mujoco.mjtObj.mjOBJ_JOINT, jnt_id)
        default_pos = get_default_joint_pos(model, jnt_name)
        default_dof_pos.append(default_pos)

    default_dof_pos = np.array(default_dof_pos, dtype=np.float32)
    actuator_joint_ids = np.array(actuator_joint_ids, dtype=np.int32)

    dt = model.opt.timestep
    print(dt)
    decimation = 5
    sim_dt = dt * decimation
    action_scale = 0.5

    last_actions = np.zeros(model.nu, dtype=np.float32)
    command = np.array([0.0, 0.0, 0.0], dtype=np.float32)

    mujoco.mj_resetData(model, data)
    data.qpos[0:3] = ZERO_POSE.pos

    for i in range(model.njnt):
        jnt_name = mujoco.mj_id2name(model, mujoco.mjtObj.mjOBJ_JOINT, i)
        if jnt_name:
            q_adr = model.jnt_qposadr[i]
            d_val = get_default_joint_pos(model, jnt_name)
            if model.jnt_type[i] == mujoco.mjtJoint.mjJNT_FREE:
                continue  # Handled by ZERO_POSE.pos
            data.qpos[q_adr] = d_val

    mujoco.mj_forward(model, data)

    with mujoco.viewer.launch_passive(model, data) as viewer:
        while viewer.is_running():
            step_start = time.time()

            base_ang_vel = data.sensor("robot/imu_ang_vel").data
            projected_gravity = -data.sensor("robot/projected_gravity").data

            current_joint_pos = []
            current_joint_vel = []

            for i in range(model.njnt):
                jnt_type = model.jnt_type[i]
                if jnt_type == mujoco.mjtJoint.mjJNT_FREE:
                    continue

                q_adr = model.jnt_qposadr[i]
                v_adr = model.jnt_dofadr[i]

                # Calculate relative position
                jnt_name = mujoco.mj_id2name(model, mujoco.mjtObj.mjOBJ_JOINT, i)
                default = get_default_joint_pos(model, jnt_name)

                current_joint_pos.append(data.qpos[q_adr] - default)
                current_joint_vel.append(data.qvel[v_adr])

            current_joint_pos = np.array(current_joint_pos, dtype=np.float32)
            current_joint_vel = np.array(current_joint_vel, dtype=np.float32)

            rel_joint_pos = current_joint_pos - default_dof_pos

            obs_list = [
                base_ang_vel,  # 3
                projected_gravity,  # 3
                rel_joint_pos,  # 20
                current_joint_vel,  # 20
                last_actions,  # 20
                command,  # 3 (assuming 3 dims for twist)
            ]

            obs = np.concatenate(obs_list).astype(np.float32)
            obs = np.expand_dims(obs, axis=0)  # Add batch dim

            actions_raw = ort_sess.run(None, {input_name: obs})[0][0]
            last_actions = actions_raw

            ctrl_target = (actions_raw * action_scale) + default_dof_pos

            data.ctrl[:] = ctrl_target

            for _ in range(decimation):
                mujoco.mj_step(model, data)

            viewer.sync()

            time_until_next_step = sim_dt - (time.time() - step_start)
            if time_until_next_step > 0:
                time.sleep(time_until_next_step)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run K1 Velocity Inference")
    parser.add_argument("model", type=str, help="Path to the ONNX policy file")
    args = parser.parse_args()

    main(args.model)
