import random
from typing import List, Tuple


class Joint():
    def __init__(self,
                 length: float,
                 base_angles: List[float],
                 pose_angles: List[Tuple],
                 parent=None):
        """
        :param length: The length of the bone
        :param base_angles: set of angles the joint is rotated by before the pose_angles are applied
        :param pose_angles: set of tuples containing an axis_index, joint_index and a multiplier
        :param parent: Another joint object that this one will be attached to
        """
        self.children = []
        self.length = length
        self.base_angles = base_angles
        self.pose_angles = pose_angles
        self.parent = parent
        if parent:
            parent.children.append(self)
