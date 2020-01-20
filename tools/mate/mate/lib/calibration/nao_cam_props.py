from enum import IntEnum
import numpy as np

from mate.lib.transforms.transforms import Transforms

class NaoCamProps(object):
    class CamSelect(IntEnum):
        NONE = 0
        TOP = 1
        BOTTOM = 2
        BOTH = 3

    TOP = CamSelect.TOP
    BOTTOM = CamSelect.BOTTOM

    CAM_ENUM_TO_STR_MAP = {
        TOP: "top",
        BOTTOM: "bottom"
    }

    CAMERAS = [key for key, val in CAM_ENUM_TO_STR_MAP.items()]
    CAM_NAMES = [val for val in CAM_ENUM_TO_STR_MAP.items()]

    # TORSO_TO_GROUND = "torso2Ground"
    # HEAD_TO_TORSO = "head2Torso"
    # CAM_TO_GROUND = "camera2Ground"

    INTRINSIC = "intrinsic"
    EXTRINSIC = "extrinsic"

    CAM_TO_HEAD_UNCALIB = {
        TOP: Transforms.getHomographyEuler(
            [0, 0.0209, 0],
            [58.71, 0, 63.64]
        ),
        BOTTOM: Transforms.getHomographyEuler(
            [0, 0.6929, 0],
            [50.71, 0, 17.74]
        )
    }
    CAM_TO_HEAD_UNCALIB_INV = {
        TOP: np.matrix(CAM_TO_HEAD_UNCALIB[TOP]).I,
        BOTTOM: np.matrix(CAM_TO_HEAD_UNCALIB[BOTTOM]).I
    }

    def __init__(self, camera: CamSelect, image_width=640, image_height=480):
        super(NaoCamProps, self)
        self.image_width = image_width
        self.image_height = image_height
        self.camera = camera
        self.ext = [0.0, 0.0, 0.0]
        self.fc = [0] * 2
        self.cc = [0] * 2
        self.setIntrinsicScaled([0.874765625, 1.1709375], [0.5, 0.5])

    def setExtrinsic(self, val):
        self.ext = val

    def setIntrinsic(self, fc, cc):
        self.fc[0] = fc[0]
        self.cc[0] = cc[0]
        self.fc[1] = fc[1]
        self.cc[1] = cc[1]

    def setIntrinsicScaled(self, fc, cc):
        self.fc[0] = fc[0] * self.image_width
        self.cc[0] = cc[0] * self.image_width
        self.fc[1] = fc[1] * self.image_height
        self.cc[1] = cc[1] * self.image_height

    @staticmethod
    def getIntrinsicScaled(fc, cc, im_width, im_height):
        un_fc = [0] * 2
        un_cc = [0] * 2
        un_fc[0] = fc[0] / im_width
        un_cc[0] = cc[0] / im_width
        un_fc[1] = fc[1] / im_height
        un_cc[1] = cc[1] / im_height
        return un_fc, un_cc

    @staticmethod
    def digestIntrinsicMat(mat):
        mat = np.matrix(mat)
        return (mat[0, 0], mat[1, 1]), (mat[0, 2], mat[1, 2])

    def getIntrinsic(self):
        return self.fc, self.cc

    def getIntrinsicMat(self):
        '''
        This give a traditional intrinsic matrix
        '''
        return np.matrix(
            [[self.fc[0], 0, self.cc[0]], [0, self.fc[1], self.cc[1]],
             [0, 0, 0]],
            dtype=float)

    @staticmethod
    def robotToPixel(fc, cc, transform, data):
        return NaoCamProps.cameraToPixel(
            fc, cc, Transforms.transformHomography(transform, data))

    @staticmethod
    def cameraToPixel(fc, cc, data):
        '''
        data must be a 3xn matrix
        Extracted from CameraMatrix.hpp
        x_img = cc.x - fc.x * Y_3D / X_3D
        y_img = cc.y - fc.y * Z_3D / X_3D
        '''
        data_shape = np.matrix(data).shape
        if data_shape[0] != 3:
            raise ValueError(
                "cameraToPixel -> Input dimensions must be 3xn matrix ",
                np.matrix(data).shape)

        successes = [False] * data_shape[1]

        for i in range(0, data_shape[1]):
            if data[0, i] > 0.0:
                successes[i] = True

        # Divide Y and Z by X
        stage_1 = data / data[0, :]

        stage_2 = [[fc[0], 0], [0, fc[1]]] * stage_1[1:3, :]
        output = [[cc[0]], [cc[1]]] - stage_2
        return output[0:2], successes

    @staticmethod
    def getExtrinsicMat(val):
        return Transforms.getRotMatEuler(np.array(val) * -1).transpose()

