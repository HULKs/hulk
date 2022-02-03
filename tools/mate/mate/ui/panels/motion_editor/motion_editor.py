from enum import Enum
import os
import typing
import copy


class Joints(Enum):
    HEAD_YAW = 0
    HEAD_PITCH = 1
    L_SHOULDER_PITCH = 2
    L_SHOULDER_ROLL = 3
    L_ELBOW_YAW = 4
    L_ELBOW_ROLL = 5
    L_WRIST_YAW = 6
    L_HAND = 7
    L_HIP_YAW_PITCH = 8
    L_HIP_ROLL = 9
    L_HIP_PITCH = 10
    L_KNEE_PITCH = 11
    L_ANKLE_PITCH = 12
    L_ANKLE_ROLL = 13
    R_HIP_YAW_PITCH = 14
    R_HIP_ROLL = 15
    R_HIP_PITCH = 16
    R_KNEE_PITCH = 17
    R_ANKLE_PITCH = 18
    R_ANKLE_ROLL = 19
    R_SHOULDER_PITCH = 20
    R_SHOULDER_ROLL = 21
    R_ELBOW_YAW = 22
    R_ELBOW_ROLL = 23
    R_WRIST_YAW = 24
    R_HAND = 25
    JOINTS_MAX = 26


def reset_model(model):
    model["valid"] = False
    model["motion2_data"] = None
    model["current_frame"] = 0
    model["t_to_reach_current_frame"] = 0.0
    model["t_to_reach_duration"] = 0
    model["is_playing"] = False
    model["is_capturing"] = False
    model["selected_joint"] = None
    model["highlight_joint_plot"] = None
    model["live_angles"] = None
    model["live_mode"] = False
    model["puppet_mode"] = False
    model["selected_joint"] = None
    model["highlight_joint_plot"] = None


def add_frame(model, angles_to_copy=None):
    index = 0
    if model["motion2_data"] is None:
        model["motion2_data"] = copy.deepcopy(model["empty_data"])
    elif len(model["motion2_data"]["position"]) != 0:
        index = model["current_frame"] + 1
    if angles_to_copy is not None:
        angles = list(angles_to_copy)
    elif model["live_mode"]:
        angles = model["live_angles"]
    else:
        angles = [0] * Joints.JOINTS_MAX.value
    model["motion2_data"]["position"].insert(index, {
        "parameters": angles,
        "time": 1000})
    model["t_to_reach_duration"] += 1000
    model["current_frame"] = index


def cut_frame_at_t(model):
    new_angles = get_interpolated_position(model)
    i = model["current_frame"]
    new_time = int(model["t_to_reach_current_frame"] * get_current_position(model)["time"])
    model["motion2_data"]["position"].insert(i, {
        "parameters": new_angles,
        "time": new_time})
    model["motion2_data"]["position"][i + 1]["time"] -= new_time
    model["t_to_reach_current_frame"] = 1.0


def get_current_position(model):
    return model["motion2_data"]["position"][model["current_frame"]]


def calculate_frame_durations(model):
    if model["valid"]:
        target_duration = model["motion2_data"]["header"]["time"]
        current_duration = get_summed_frame_duration(model)
        per_frame_unit = float(target_duration) / float(current_duration)
        for i in range(len(model["motion2_data"]["position"])):
            model["motion2_data"]["position"][i]["time"] = int(
                model["motion2_data"]["position"][i]["time"] *
                per_frame_unit)


def calculate_motion_duration(model):
    model["motion2_data"]["header"]["time"] = get_summed_frame_duration(model)


def get_previous_position(model):
    if model["current_frame"] == 0:
        return model["motion2_data"]["position"][0]
    else:
        return model["motion2_data"]["position"][model["current_frame"] - 1]


def get_interpolated_position(model):
    prev = get_previous_position(model)["parameters"]
    curr = get_current_position(model)["parameters"]
    t = model["t_to_reach_current_frame"]
    return [((1-t)*p) + (t*c) for (p, c) in zip(prev, curr)]


def get_current_angles_sorted(model):
    angles = [0] * 26
    interpolated_position = get_interpolated_position(model)
    for index, joint_index in enumerate(
            model["motion2_data"]["header"]["joints"]):
        angles[joint_index] = interpolated_position[index]
    return angles


def get_joint_index(model, joint: Joints):
    if model["live_mode"]:
        return joint.value

    for i, value in enumerate(model["motion2_data"]["header"]["joints"]):
        if value == joint.value:
            return i
    raise ValueError("Joint {} not found".format(joint.name))


def get_summed_frame_duration(model):
    duration = 0
    for frame in model["motion2_data"]["position"]:
        duration += frame["time"]
    return duration


def get_absolute_frame_time(model, index: int):
    all_times = 0
    for frame in model["motion2_data"]["position"]:
        all_times += frame["time"]
    relative_time = float(
        model["motion2_data"]["position"][index]["time"]) / float(all_times)
    return int(model["motion2_data"]["header"]["time"] * relative_time)


def get_delta(model):
    delta = 0
    frame_i = model["current_frame"]
    last_angles = model["motion2_data"]["position"][frame_i]["parameters"]
    for i in range(len(last_angles)):
        delta += abs(last_angles[i] - model["live_angles"][i])
    return delta
