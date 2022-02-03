from .motion_editor import Joints
from .joint import Joint
import random


class NaoRig():
    def __init__(self, x=0, y=0):
        self.body = Joint(0, [0, 180, 0], [])
        torso = Joint(126, [0, 0, 0], [], self.body)
        head = Joint(60, [0, 180, 0], [(1, Joints.HEAD_YAW),
                                       (0, Joints.HEAD_PITCH)], torso)
        # lower body
        pelvis_center = Joint(-35, [0, 0, 0], [], self.body)
        pelvis_left = Joint(70, [0, 0, 135], [(1, Joints.L_HIP_YAW_PITCH)],
                            pelvis_center)
        pelvis_right = Joint(70, [0, 0, -135],
                             [(1, Joints.R_HIP_YAW_PITCH, -1)], pelvis_center)
        left_thigh = Joint(100, [0, 0, 45], [(0, Joints.L_HIP_PITCH),
                                             (2, Joints.L_HIP_ROLL, -1)],
                           pelvis_left)
        right_thigh = Joint(100, [0, 0, -45], [(0, Joints.R_HIP_PITCH),
                                               (2, Joints.R_HIP_ROLL, -1)],
                            pelvis_right)
        left_tibia = Joint(102, [0, 0, 0], [(0, Joints.L_KNEE_PITCH)],
                           left_thigh)
        right_tibia = Joint(102, [0, 0, 0], [(0, Joints.R_KNEE_PITCH)],
                            right_thigh)
        left_ankle = Joint(-70, [90, 0, 0], [(0, Joints.L_ANKLE_PITCH),
                                             (1, Joints.L_ANKLE_ROLL, -1)],
                           left_tibia)
        right_ankle = Joint(-70, [90, 0, 0], [(0, Joints.R_ANKLE_PITCH),
                                              (1, Joints.R_ANKLE_ROLL, -1)],
                            right_tibia)
        # upper body
        left_shoulder = Joint(98, [0, 0, 90], [], torso)
        right_shoulder = Joint(98, [0, 0, -90], [], torso)
        left_arm = Joint(100, [0, 90, -90], [(0, Joints.L_SHOULDER_PITCH, -1),
                                             (2, Joints.L_SHOULDER_ROLL)],
                         left_shoulder)
        right_arm = Joint(100, [0, 90, -90], [(0, Joints.R_SHOULDER_PITCH),
                                              (2, Joints.R_SHOULDER_ROLL, -1)],
                          right_shoulder)
        left_lower_arm = Joint(55, [0, 90, 0], [(1, Joints.L_ELBOW_YAW),
                                                (0, Joints.L_ELBOW_ROLL, -1)],
                               left_arm)
        right_lower_arm = Joint(55, [0, 90, 0], [(1, Joints.R_ELBOW_YAW),
                                                 (0, Joints.R_ELBOW_ROLL)],
                                right_arm)
        left_hand = Joint(57, [0, 0, 0], [(1, Joints.L_WRIST_YAW)],
                          left_lower_arm)
        right_hand = Joint(57, [0, 0, 0], [(1, Joints.R_WRIST_YAW)],
                           right_lower_arm)
