import numpy as np

ROBOT_TO_LEFT_PELVIS = np.array([0.0, 0.05, 0.0])
ROBOT_TO_RIGHT_PELVIS = np.array([0.0, -0.05, 0.0])
HIP_TO_KNEE = np.array([0.0, 0.0, -0.1])
KNEE_TO_ANKLE = np.array([0.0, 0.0, -0.1029])
ANKLE_TO_SOLE = np.array([0.0, 0.0, -0.04519])
