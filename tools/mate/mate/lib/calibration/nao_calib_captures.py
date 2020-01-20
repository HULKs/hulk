import numpy as np
from enum import IntEnum

from PyQt5.QtGui import QPixmap as QPixmap

import mate.net.nao_data as nd
from mate.lib.transforms.transforms import Transforms

from .nao_cam_props import *

class NaoKinematicMatrixCapture:

    class DataKey(IntEnum):
        TIMESTAMP = 0,
        TORSO_TO_GROUND = 1,
        HEAD_TO_TORSO = 2,
        CAM_TO_GROUND = 3,
        GROUND_TO_TORSO = 4,
        TORSO_TO_HEAD = 5,
        GROUND_TO_CAM = 6

    DATA_KEY_TO_STR = {
        DataKey.TIMESTAMP: "timestamp",
        DataKey.TORSO_TO_GROUND: "torso2Ground",
        DataKey.HEAD_TO_TORSO: "head2Torso",
        DataKey.CAM_TO_GROUND: "camera2Ground",
        DataKey.GROUND_TO_TORSO: "ground2Torso",
        DataKey.TORSO_TO_HEAD: "torso2Head",
        DataKey.GROUND_TO_CAM: "ground2Camera"
    }

    @staticmethod
    def getDataKeyString(key: DataKey):
        return NaoKinematicMatrixCapture.DATA_KEY_TO_STR[key]

    def __init__(self, camera, timestamp=-1, torso2Head=np.eye(4), ground2Torso=np.eye(4), ground2Camera=np.eye(4)):
        self.torso_to_head = np.matrix(torso2Head)
        self.ground_to_torso = np.matrix(ground2Torso)
        self.ground_to_camera = np.matrix(ground2Camera)

        self.timestamp = timestamp
        self.camera = camera

    def setValues(self, camera, timestamp, torso2Head, ground2Torso, ground2Camera):
        if camera != self.camera:
            raise ValueError("Wrong camera!!!")
        self.timestamp = timestamp
        self.torso_to_head = np.mat(torso2Head)
        self.ground_to_torso = np.mat(ground2Torso)
        self.ground_to_camera = np.mat(ground2Camera)

class NaoCaptureData(object):
    '''
    This is a container for holding information of each snapshot
    '''

    def __init__(self, camera: NaoCamProps.CamSelect, kinematicCap: None):
        super(NaoCaptureData, self).__init__()
        if camera not in NaoCamProps.CAMERAS:
            raise ValueError(
                "Camer name must be either top or bottom!" + str(camera))
        self.camera = camera

        self.kinematic_data = kinematicCap if kinematicCap else NaoKinematicMatrixCapture(
            camera)

        # In case of charuco, id = charuco point
        # In case of charuco diamond, id = combination of marker ID's
        self.marker_ids = []
        # If aruco or diamond, 3x4n matrix where n = detected markers (or diamonds)
        # If charuco, 3xn matrix where n = detected charuco (chessboard) corners
        self.detected_points = []

        # In robot's ground frame
        self.board_points_3D = []

        # This is to facilitate array slicing between each board when it comes to multiple poses per board
        self.points_per_board = []


class ImageWithKinematicData(nd.DebugImage):
    '''
    This class will hold the individual "snapshot" sent by nao when triggered via
    calibrationCaptureTrigger config param
    TODO FUTURE -> Make to hold joint angle data
    '''
    TIMEOUT = 20  # ms

    def __init__(self, camera, key: str, width: int, height: int, data: bytes, update: bool = False):
        super(ImageWithKinematicData, self).__init__(key, width, height, data)

        self.camera = camera
        self.kinematic_chain = NaoKinematicMatrixCapture(camera)
        self.is_img_dat_updated = update  # make true when new data is there
        self.is_kinematics_updated = False
        self.isSynced = False

    def reset(self, image: nd.DebugImage):
        self.data = image.data
        self.timestamp = image.timestamp
        self.width = image.width
        self.height = image.height

        self.is_img_dat_updated = True
        self.is_kinematics_updated = False
        self.isSynced = False

    def loadToPixMap(self, pixmap: QPixmap):
        pixmap.loadFromData(self.data)
        self.is_img_dat_updated = False

    def setKinematicChain(self, camera, head2Torso, torso2Ground, camera2Ground):
        camera2Ground[0:3, 3] *= 1000  # convert to mm
        self.kinematic_chain.setKinematicChain(camera, np.matrix(head2Torso).I,
                                               np.matrix(torso2Ground).I, np.matrix(camera2Ground).I)
        self.is_kinematics_updated = True

    def getNaoCapData(self):
        return NaoCaptureData(self.camera, self.kinematic_chain)

    @staticmethod
    def naoDebugKinMatrixToAffine(data):
        axisAngle = data[0][:]
        tvec = data[1][:]
        return Transforms.axTransToAffine(axisAngle, tvec)