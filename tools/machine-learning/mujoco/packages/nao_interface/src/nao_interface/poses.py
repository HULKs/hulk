from nao_interface.joints import ArmJoints, HeadJoints, Joints, LegJoints

PENALIZED_POSE = {
    "head": {"pitch": 0.0, "yaw": 0.0},
    "left_arm": {
        "elbow_roll": 0.0,
        "elbow_yaw": -1.57,
        "hand": 0.0,
        "shoulder_pitch": 1.57,
        "shoulder_roll": 0.1,
        "wrist_yaw": 0.0,
    },
    "left_leg": {
        "ankle_pitch": 0.01,
        "ankle_roll": -0.002,
        "hip_pitch": 0.09,
        "hip_roll": 0.0,
        "hip_yaw_pitch": 0.0,
        "knee_pitch": -0.06,
    },
    "right_arm": {
        "elbow_roll": 0.0,
        "elbow_yaw": 1.57,
        "hand": 0.0,
        "shoulder_pitch": 1.57,
        "shoulder_roll": -0.1,
        "wrist_yaw": 0.0,
    },
    "right_leg": {
        "ankle_pitch": 0.01,
        "ankle_roll": 0.002,
        "hip_pitch": 0.09,
        "hip_roll": 0.0,
        "knee_pitch": -0.06,
    },
}

ZERO_POSE = {
    "head": {"pitch": 0.0, "yaw": 0.0},
    "left_arm": {
        "elbow_roll": 0.0,
        "elbow_yaw": 0.0,
        "hand": 0.0,
        "shoulder_pitch": 0.0,
        "shoulder_roll": 0.0,
        "wrist_yaw": 0.0,
    },
    "left_leg": {
        "ankle_pitch": 0.0,
        "ankle_roll": 0.0,
        "hip_pitch": 0.0,
        "hip_roll": 0.0,
        "hip_yaw_pitch": 0.0,
        "knee_pitch": 0.0,
    },
    "right_arm": {
        "elbow_roll": 0.0,
        "elbow_yaw": 0.0,
        "hand": 0.0,
        "shoulder_pitch": 0.0,
        "shoulder_roll": 0.0,
        "wrist_yaw": 0.0,
    },
    "right_leg": {
        "ankle_pitch": 0.0,
        "ankle_roll": 0.0,
        "hip_pitch": 0.0,
        "hip_roll": 0.0,
        "knee_pitch": 0.0,
    },
}

READY_POSE = Joints(
    head=HeadJoints(pitch=0.0, yaw=0.0),
    left_arm=ArmJoints(
        elbow_roll=0.0,
        elbow_yaw=-1.57,
        shoulder_pitch=1.57,
        shoulder_roll=0.1,
        wrist_yaw=0.0,
    ),
    right_arm=ArmJoints(
        elbow_roll=0.0,
        elbow_yaw=1.57,
        shoulder_pitch=1.57,
        shoulder_roll=-0.1,
        wrist_yaw=0.0,
    ),
    hip_yaw_pitch=0.0,
    left_leg=LegJoints(
        hip_roll=0.010821502783627257,
        hip_pitch=-0.3107718500421241,
        knee_pitch=0.8246279211008891,
        ankle_pitch=-0.513856071058765,
        ankle_roll=-0.010821502783627453,
    ),
    right_leg=LegJoints(
        hip_roll=-0.010821502783627257,
        hip_pitch=-0.3107718500421241,
        knee_pitch=0.8246279211008891,
        ankle_pitch=-0.513856071058765,
        ankle_roll=0.010821502783627453,
    ),
)
