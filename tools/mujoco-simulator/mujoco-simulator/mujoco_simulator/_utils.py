import numpy as np
from scipy.spatial.transform import Rotation


def mj_quaternion_to_rpy(q_wxyz: np.ndarray) -> np.ndarray:
    # Convert wxyz to xyzw
    q_xyzw = np.roll(q_wxyz, -1)
    rotation = Rotation.from_quat(q_xyzw)
    return rotation.as_euler("xyz", degrees=False)
