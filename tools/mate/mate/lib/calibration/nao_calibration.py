'''
Nao specific calibration stuff. This might be re-designed as an implementation of GenericCalibration

__author__ = "Darshana Adikari"
__copyright__ = "Copyright 2018, RobotING@TUHH / HULKs"
__license__ = ""
__version__ = "0.2"
__maintainer__ = "Darshana Adikari"
__email__ = "darshana.adikari@tuhh.de, darshanaads@gmail.com"
__status__ = "Alpha"
'''

from enum import IntEnum
import time
import json

import numpy as np
from scipy.optimize import *

from transforms3d import axangles, affines
from .calibration import *

# TODO Move this to calibration.py
import cv2.aruco as ar


class NaoCamProps(object):
    class CamSelect(IntEnum):
        NONE = 0
        TOP = 1
        BOTTOM = 2
        BOTH = 3

    TOP = "top"
    BOTTOM = "bottom"
    TORSO_TO_GROUND = "Torso2Ground"
    HEAD_TO_TORSO = "Head2Torso"
    CAM_TO_GROUND = "Camera2Ground"

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

    def __init__(self, cam_name: CamSelect, image_width=640, image_height=480):
        super(NaoCamProps, self)
        self.image_width = image_width
        self.image_height = image_height
        self.cam_name = cam_name
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


class NaoCalibrationResult(object):
    TOP_EXT = "top_ext"
    BOTTOM_EXT = "bottom_ext"
    TOP_FC = "top_fc"
    BOTTOM_FC = "bottom_fc"
    TOP_CC = "top_cc"
    BOTTOM_CC = "bottom_cc"
    MOUNT = "Brain.Projection"

    def __init__(self):
        super(NaoCalibrationResult, self)
        self.is_ext_top_done = False
        self.is_ext_bottom_done = False
        self.is_int_top_done = False
        self.is_int_bottom_done = False
        self.top_ext = [0, 0, 0]
        self.bottom_ext = [0, 0, 0]
        self.top_fc = [0, 0]
        self.top_cc = [0, 0]
        self.bottom_fc = [0, 0]
        self.bottom_cc = [0, 0]

    def setTopIntrinsics(self, fc, cc):
        self.top_fc = fc
        self.top_cc = cc

    def setBottomIntrinsics(self, fc, cc):
        self.bottom_fc = fc
        self.bottom_cc = cc

    def encodeExtrinsicCalibparams(self, settings, tuning_params):
        '''
        settings = [
            is_top_ext
            is_bot_ext
            top_offset
            bottom_offset
            is_torso
        ]
        '''
        print(settings)
        # trim the tuning params
        if not settings[2]:  # no top data!
            settings[0] = False
            tuning_params = tuning_params[0:3]
            if (settings[3] - settings[2] <= 0):  # no bottom data!

                settings[1] = False
                tuning_params = []
        else:
            if (settings[3] - settings[2]) <= 0:  # no bottom data!
                settings[1] = False
                tuning_params = tuning_params[0:3]
        if settings[4]:
            tuning_params.append(0)
            tuning_params.append(0)

        return settings, tuning_params

    def decodeExtrinsicCalibParams(self, settings, extrinsic_output):
        bottom_param_offset = 0
        if settings[0]:
            self.is_ext_top_done = True
            # first 3 values
            self.top_ext = extrinsic_output[0:3]
            bottom_param_offset = 3
        if settings[1]:
            self.is_ext_bottom_done = True
            # first 3 values
            self.bottom_ext = extrinsic_output[bottom_param_offset:
                                               bottom_param_offset + 3]

        # TORSO stuff.
        if settings[4]:
            pass


class NaoCalibSettings(object):
    def __init__(self,
                 boards: [BoardProperties],
                 int_top: bool = False,
                 int_bot: bool = False,
                 ext_top: bool = False,
                 ext_bot: bool = False,
                 joint_calib: bool = False):
        super(NaoCalibSettings, self).__init__()

        self.is_intrinsic_top = int_top
        self.is_intrinsic_bottom = int_bot
        self.is_extrinsic_top = ext_top
        self.is_extrinsic_bottom = ext_bot
        self.is_joint_calib = joint_calib
        self.is_torso_calib = False

        self.projectionConfig = {"torso_calibration": [0, 0]}
        '''
        Intrinsic calibration needs many points -> ChAruco board with 4x10 or so
        Extrinsic with arbitrary placed patterns best benefit from -> ChAruco Diamond (3x3 ch. board) that can have multiple appearances
        '''
        if len(boards) > 1 and any(board.board == None or board.pattern_type ==
                                   BoardProperties.PatternType.CHARUCO_BOARD
                                   for board in boards):
            raise RuntimeError(
                "Only one Charuco or Aruco board can exist EXCEPT 3x3 ChAruco Diamond patterns which are allowed."
            )
        self.boards = boards

    def setFlags(self,
                 int_top: bool = False,
                 int_bot: bool = False,
                 ext_top: bool = False,
                 ext_bot: bool = False,
                 joint_calib: bool = False,
                 torso_calib=False):
        self.is_intrinsic_top = int_top
        self.is_intrinsic_bottom = int_bot
        self.is_extrinsic_top = ext_top
        self.is_extrinsic_bottom = ext_bot
        self.is_joint_calib = joint_calib
        self.is_torso_calib = False


class NaoCaptureData(object):
    '''
    This is a container for holding information of each snapshot
    '''

    def __init__(self, cam_name: NaoCamProps.CamSelect):
        super(NaoCaptureData, self).__init__()
        if cam_name != NaoCamProps.CamSelect.TOP and cam_name != NaoCamProps.CamSelect.BOTTOM:
            raise ValueError("Camer name must be either top or bottom!")
        self.cameraName = cam_name
        self.torso_to_head = np.empty((4, 4))
        self.ground_to_torso = np.empty((4, 4))
        self.camera_to_ground = np.empty((4, 4))

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


class NaoCalibration(object):
    # FUTURE ARUCO_RANDOM

    def __init__(self,
                 calib_settings: NaoCalibSettings,
                 image_width=640,
                 image_height=480):
        super(NaoCalibration, self).__init__()
        self.settings = calib_settings

        #### capture data ####
        self.top_data = []  # NaoCaptureData(NaoCamProps.TOP)
        self.bottom_data = []  # NaoCaptureData(NaoCamProps.BOTTOM)
        self.top_camera = NaoCamProps(NaoCamProps.CamSelect.TOP, image_width,
                                      image_height)
        self.bottom_camera = NaoCamProps(NaoCamProps.CamSelect.BOTTOM,
                                         image_width, image_height)

        self.calibrator = GenericCalibration(self.settings.boards)

    def clearCapturedData(self):
        self.top_data = []
        self.bottom_data = []

    def captureDataToJsonFile(self, fileName):
        with open(str(fileName), 'w') as outfile:
            json.dump(
                {
                    "top": self.top_data,
                    "bottom": self.bottom_data
                },
                outfile,
                indent=2)

    def processCapture(self, capture_data: NaoCaptureData, image):
        '''
        It'll be nice if this step can be done asynchronously from the UI thread.
        Preferably a queue of sorts into the calibration library?
        '''

        # decode the array into an image
        # TODO Future; auto detect?
        # startTime = time.perf_counter()
        img = cv.imdecode(
            np.fromstring(image, dtype='uint8'), cv.IMREAD_UNCHANGED)

        all_detected_corners, all_board_corners, all_ids, points_per_board = self.extractCalibPoints(
            img)
        if len(all_ids):
            capture_data.board_points_3D = all_board_corners
            capture_data.detected_points = all_detected_corners
            # Below disabled for performance reasons. May need enabling later
            capture_data.marker_ids = all_ids
            capture_data.points_per_board = points_per_board

            if capture_data.cameraName == NaoCamProps.CamSelect.TOP:
                self.top_data.append(capture_data)
            elif capture_data.cameraName == NaoCamProps.CamSelect.BOTTOM:
                self.bottom_data.append(capture_data)
            else:
                raise ValueError("Capture data got incorrect camera name")
        return len(self.top_data), len(self.bottom_data)

    def getGroundToCamera(self, captureData: NaoCaptureData, ext_param):
        ground2camgen = np.matmul(
            NaoCamProps.CAM_TO_HEAD_UNCALIB_INV[NaoCamProps.TOP],
            np.matmul(captureData.torso_to_head, captureData.ground_to_torso))

        return ground2camgen

    def projectBoardPoints(self,
                           capture_data: NaoCaptureData,
                           camera: NaoCamProps,
                           groundToCamera,
                           ext=None):
        '''
        Convenience function. Not to be used in high frequencies
        returns an array of type: (id, detected_pt, proj_pt)
        '''

        fc, cc = camera.getIntrinsic()

        if not ext:
            ext = camera.ext

        extrinsicMat = camera.getExtrinsicMat(ext)

        pt_in_cam_uncalib = Transforms.transformHomography(
            Transforms.kinematicInv(capture_data.camera_to_ground),
            capture_data.board_points_3D)

        pt_in_cam = Transforms.transformHomography(extrinsicMat,
                                                   pt_in_cam_uncalib)

        twoDeepts, successes = NaoCamProps.robotToPixel(
            fc, cc, extrinsicMat, pt_in_cam)

        ids_copy = np.ravel(capture_data.marker_ids)
        data = [None] * len(ids_copy)
        for idx, elem in enumerate(data):
            data[idx] = (ids_copy[idx], (twoDeepts[0, idx], twoDeepts[1, idx]))
        return data

    def extractCalibPoints(self, image):
        try:
            res = self.calibrator.extractCalibPoints(image)
            return res
        except Exception as e:
            print("Exception happened :o", e)

    # Consider on static or not!
    @staticmethod
    def extrinsicCostFunc(params, *args):
        '''
        @param params - tunables
        @args   0. independant variables (3D board corners)
                1. measurements (detected corners)
                2. intrisic_params (array of tupels -> [(fc,cc)], 0 = top, 1 = bottom)
                3. optimizer settings - choose which camera, etc.
                    - ext_top, ext_bottom, torso.
                    - currently just a set of ints or floats
        '''
        if (len(args) != 4) or len(args[2]) != 2:
            raise ValueError("Incorrect amount of args supplied")

        detectedCorners = args[1]
        intrisic_params = args[2]
        settings = args[3]

        isTopEnabled = True if settings[0] else False
        isBottomEnabled = True if settings[1] else False
        top_sample_offset = settings[2]
        bottom_sample_offset = settings[3]
        isTorsoEnabled = True if settings[4] else False

        topParamEnd = 3 if isTopEnabled else 0
        bottomParamEnd = topParamEnd + 3 if isBottomEnabled else topParamEnd
        torsoParamEnd = bottomParamEnd + 2 if isTorsoEnabled else bottomParamEnd

        if len(params) < torsoParamEnd:
            raise ValueError("Not enough parameters supplied!!")

        residual = np.empty((2, 0))

        if isTorsoEnabled:
            # TODO finish this or leave unless needed
            raise NotImplementedError("torso calib. not implemented yet")
            torsoMatrix = Transforms.getRotMatEuler(
                (params[bottomParamEnd:bottomParamEnd + 2] + [0]))
        else:
            boardCorners_wrt_cam_u = args[0]
            residual = np.empty((boardCorners_wrt_cam_u.shape[1] * 2, 1))
            begin_offset = None
            end_offset = None
            ext_param = [None] * 3
            int_param = [None] * 2

            for i in [0, 1]:
                if (i == 0 and isTopEnabled):
                    begin_offset = 0
                    end_offset = top_sample_offset
                    ext_param = params[:topParamEnd]
                    int_param = intrisic_params[0]
                elif (i == 1 and isBottomEnabled):
                    begin_offset = top_sample_offset
                    end_offset = bottom_sample_offset
                    ext_param = params[topParamEnd:bottomParamEnd]
                    int_param = intrisic_params[1]
                else:
                    continue

                begin_offset_2x = 2 * begin_offset
                end_offset_2x = 2 * end_offset

                # we need inverse of extrinsic matrix = transpose!
                extrinsic = NaoCamProps.getExtrinsicMat(ext_param).transpose()
                proj_points = NaoCamProps.robotToPixel(
                    int_param[0], int_param[1], extrinsic,
                    boardCorners_wrt_cam_u[:, begin_offset:end_offset])

                # TODO  Do the proj_points part in a more elegant manner!
                np.subtract(detectedCorners[begin_offset_2x:end_offset_2x],
                            proj_points[0].transpose().ravel().tolist()[0],
                            residual[begin_offset_2x:end_offset_2x, 0])
        if len(residual) != len(detectedCorners):
            raise ValueError("Residual cols and input cols doesnt match",
                             residual.shape, detectedCorners.shape)
        return residual.ravel()

    def verifyValues(self):
        if (len(self.top_data) + len(self.bottom_data)) <= 0:
            return False
        return True

    def startCalibration(self):
        print("\nStarting Calibration preperations... \n")
        startTime = time.perf_counter()

        # TODO Implement this!
        if not self.verifyValues():
            ...
        result = NaoCalibrationResult()

        #### Intrinsic Calibration ####

        if self.settings.is_intrinsic_top or self.settings.is_intrinsic_bottom:
            # TODO Move the aruco part to calibration.py
            print("Starting Intrinsic")
            # We only bother with the first board :P
            board_idx = next((i for i, board in enumerate(self.settings.boards)
                              if board.pattern_type ==
                              BoardProperties.PatternType.CHARUCO_BOARD), -1)

            if board_idx < 0:
                raise ValueError(
                    "A suitable board for intrinsic calib. wasn't found!")
            # TODO properly handle this later
            if len(self.settings.boards) > 1:
                raise NotImplementedError(
                    "Multiple boards not supported for intrinsic.")

            for i in [NaoCamProps.CamSelect.TOP, NaoCamProps.CamSelect.BOTTOM]:
                if self.settings.is_intrinsic_top and i == NaoCamProps.CamSelect.TOP:
                    corners_of_all_frames = []
                    ids_of_all_frames = []

                    im_width = self.top_camera.image_width
                    im_height = self.top_camera.image_height

                    for v in self.top_data:
                        if not v.points_per_board[board_idx]:
                            continue
                        ids = v.marker_ids[board_idx]
                        detected_pts = v.detected_points[board_idx]
                        ids_of_all_frames.append(ids)
                        corners_of_all_frames.append(detected_pts)

                    if len(ids_of_all_frames):
                        cam_matrix = self.top_camera.getIntrinsicMat()
                        dist_coeffs = np.array([0, 0, 0, 0], dtype=float)
                        output = ar.calibrateCameraCharucoExtended(
                            corners_of_all_frames, ids_of_all_frames,
                            self.settings.boards[board_idx].board,
                            (im_width, im_height), cam_matrix, dist_coeffs)
                        print("top intrinsic -> retval, matrix", output[0],
                              output[1], "\n")  # , dist_coeffs, output[2]
                        result.is_int_top_done = True
                        fc, cc = NaoCamProps.digestIntrinsicMat(output[1])
                        fc, cc = NaoCamProps.getIntrinsicScaled(
                            fc, cc, im_width, im_height)
                        result.setTopIntrinsics(fc, cc)

                if self.settings.is_intrinsic_bottom and i == NaoCamProps.CamSelect.BOTTOM:
                    corners_of_all_frames = []
                    ids_of_all_frames = []

                    im_width = self.bottom_camera.image_width
                    im_height = self.bottom_camera.image_height

                    for v in self.bottom_data:
                        if not v.points_per_board[board_idx]:
                            continue
                        ids = v.marker_ids[board_idx]
                        detected_pts = v.detected_points[board_idx]
                        ids_of_all_frames.append(ids)
                        corners_of_all_frames.append(detected_pts)

                    if len(ids_of_all_frames):
                        cam_matrix = self.bottom_camera.getIntrinsicMat()
                        dist_coeffs = np.array([0, 0, 0, 0], dtype=float)
                        output = ar.calibrateCameraCharucoExtended(
                            corners_of_all_frames, ids_of_all_frames,
                            self.settings.boards[board_idx].board,
                            (im_width, im_height), cam_matrix, dist_coeffs)
                        print("bottom intrinsic -> retval, matrix", output[0],
                              output[1], "\n")

                        result.is_int_bottom_done = True
                        fc, cc = NaoCamProps.digestIntrinsicMat(output[1])
                        fc, cc = NaoCamProps.getIntrinsicScaled(
                            fc, cc, im_width, im_height)
                        result.setBottomIntrinsics(fc, cc)

            print("Intrinsic Calibration time-> ",
                  (time.perf_counter() - startTime) * 1000, "ms")
        #### Extrinsic Calibration ####
        startTime = time.perf_counter()
        if self.settings.is_torso_calib:
            raise NotImplementedError("torso calib. not implemented yet")
        else:
            top_offset = 0
            bottom_offset = 0
            independants = np.empty((3, 0))
            measurements = []  # np.empty((2, 0))
            tuning_params = []
            intrinsic_params = [None] * 2

            tpts_sum = 0
            bpts_sum = 0
            # If ext top
            if self.settings.is_extrinsic_top:
                intrinsic_params[0] = self.top_camera.getIntrinsic()
                tuning_params += self.top_camera.ext

                for v in self.top_data:
                    ground2camgen = np.matmul(
                        NaoCamProps.CAM_TO_HEAD_UNCALIB_INV[NaoCamProps.TOP],
                        np.matmul(v.torso_to_head, v.ground_to_torso))
                    val_wrt_cam_u = Transforms.transformHomography(
                        ground2camgen, v.board_points_3D)
                    independants = np.concatenate(
                        (independants, val_wrt_cam_u), axis=1)

                    for elem in v.detected_points:
                        measurements = np.append(measurements, elem.ravel())

                    top_offset += val_wrt_cam_u.shape[1]
                    tpts_sum += sum(v.points_per_board)

            bottom_offset = int(top_offset)

            # if ext. Bottom
            if self.settings.is_extrinsic_bottom:
                intrinsic_params[1] = self.bottom_camera.getIntrinsic()
                tuning_params += self.bottom_camera.ext
                for v in self.bottom_data:
                    ground2camgen = np.matmul(
                        NaoCamProps.CAM_TO_HEAD_UNCALIB_INV[
                            NaoCamProps.BOTTOM],
                        np.matmul(v.torso_to_head, v.ground_to_torso))
                    val_wrt_cam_u = Transforms.transformHomography(
                        ground2camgen, v.board_points_3D)
                    independants = np.concatenate(
                        (independants, val_wrt_cam_u), axis=1)

                    for elem in v.detected_points:
                        measurements = np.append(measurements, elem.ravel())

                    # Update by width of the 3xN matrix
                    bottom_offset += val_wrt_cam_u.shape[1]
                    bpts_sum += sum(v.points_per_board)

            # settings for cost func.
            settings = [
                self.settings.is_extrinsic_top,
                self.settings.is_extrinsic_bottom, top_offset, bottom_offset,
                self.settings.is_torso_calib
            ]
            print(settings, tuning_params)
            # Refine settings and tuning params
            settings, tuning_params = result.encodeExtrinsicCalibparams(
                settings, tuning_params)

            print("Start Extrinsic Phase:\n\tis_ext_top: ", settings[0],
                  ", is_ext_bot: ", settings[1], ", is_torso: ", settings[4],
                  settings[2], settings[3])

            if len(tuning_params) > 0:
                # Finally, invoke the solver
                print("Start minimize")

                optim_output = leastsq(
                    NaoCalibration.extrinsicCostFunc,
                    tuning_params,
                    args=(
                        independants, measurements, intrinsic_params, settings
                    )  # , method='lm'# , method='Nelder-Mead'  # , method='hybr'
                )
                result.decodeExtrinsicCalibParams(settings,
                                                  list(optim_output[0]))
            else:
                print("No suitable points, extrinsic skipped")

            print("Extrinsic Calibration time-> ",
                  (time.perf_counter() - startTime) * 1000, "ms")

        return result
