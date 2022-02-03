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
import typing
import numpy as np
from scipy.optimize import *

from transforms3d import axangles, affines
from .calibration import *

# TODO Move this to calibration.py
import cv2.aruco as ar

from mate.debug.colorlog import ColorLog
from mate.lib.calibration.nao_cam_props import NaoCamProps

logger = ColorLog()


class NaoCalibrationResult(object):
    TOP_EXT = "top_ext"
    BOTTOM_EXT = "bottom_ext"
    TOP_FC = "top_fc"
    BOTTOM_FC = "bottom_fc"
    TOP_CC = "top_cc"
    BOTTOM_CC = "bottom_cc"
    MOUNT = "Brain.Projection"

    EXT_NAMES = {
        NaoCamProps.TOP: TOP_EXT,
        NaoCamProps.BOTTOM: BOTTOM_EXT
    }

    INT_FC_NAME = {
        NaoCamProps.TOP: TOP_FC,
        NaoCamProps.BOTTOM: BOTTOM_FC
    }

    INT_CC_NAME = {
        NaoCamProps.TOP: TOP_CC,
        NaoCamProps.BOTTOM: BOTTOM_CC
    }

    def __init__(self):
        super(NaoCalibrationResult, self)
        self.is_ext_done = {
            NaoCamProps.TOP: False,
            NaoCamProps.BOTTOM: False
        }
        self.is_int_done = {
            NaoCamProps.TOP: False,
            NaoCamProps.BOTTOM: False
        }
        self.results = {
            self.TOP_EXT: [0, 0, 0],
            self.BOTTOM_EXT: [0, 0, 0],
            self.TOP_FC: [1, 1],
            self.BOTTOM_FC: [1, 1],
            self.TOP_CC: [0.5, 0.5],
            self.BOTTOM_CC: [0.5, 0.5]
        }

    def getExt(self, cam):
        return self.results[self.EXT_NAMES[cam]]

    def setTopIntrinsics(self, fc, cc):
        self.results[self.TOP_FC] = fc
        self.results[self.TOP_CC] = cc

    def setBottomIntrinsics(self, fc, cc):
        self.results[self.BOTTOM_FC] = fc
        self.results[self.BOTTOM_CC] = cc

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
        # trim the tuning params
        if not settings[2]:  # no top data
            settings[0] = False
            tuning_params = tuning_params[0:3]
            if (settings[3] - settings[2] <= 0):  # no bottom data

                settings[1] = False
                tuning_params = []
        else:
            if (settings[3] - settings[2]) <= 0:  # no bottom data
                settings[1] = False
                tuning_params = tuning_params[0:3]
        if settings[4]:
            tuning_params.append(0)
            tuning_params.append(0)

        return settings, tuning_params

    def decodeExtrinsicCalibParams(self, settings, extrinsic_output):
        bottom_param_offset = 0
        if settings[0]:
            self.is_ext_done[NaoCamProps.TOP] = True
            # first 3 values
            self.results[self.TOP_EXT] = extrinsic_output[0:3]
            bottom_param_offset = 3
        if settings[1]:
            self.is_ext_done[NaoCamProps.BOTTOM] = True
            # first 3 values
            self.results[self.BOTTOM_EXT] = extrinsic_output[bottom_param_offset:
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

        self.is_intrinsic = {
            NaoCamProps.TOP: int_top,
            NaoCamProps.BOTTOM: int_bot
        }
        self.is_extrinsic = {
            NaoCamProps.TOP: ext_top,
            NaoCamProps.BOTTOM: ext_bot
        }
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

        self.is_intrinsic[NaoCamProps.TOP] = int_top
        self.is_intrinsic[NaoCamProps.BOTTOM] = int_bot
        self.is_extrinsic[NaoCamProps.TOP] = ext_top
        self.is_extrinsic[NaoCamProps.BOTTOM] = ext_bot
        self.is_joint_calib = joint_calib
        self.is_torso_calib = False


class NaoCalibration(object):
    # FUTURE ARUCO_RANDOM

    def __init__(self,
                 calib_settings: NaoCalibSettings,
                 image_width=640,
                 image_height=480):
        super(NaoCalibration, self).__init__()
        self.settings = calib_settings

        #### capture data ####
        self.capture_data = {
            NaoCamProps.TOP: [],
            NaoCamProps.BOTTOM: []
        }
        self.camerasProperties = {
            NaoCamProps.TOP: NaoCamProps(NaoCamProps.CamSelect.TOP, image_width,
                                         image_height),
            NaoCamProps.BOTTOM: NaoCamProps(
                NaoCamProps.CamSelect.BOTTOM, image_width, image_height)
        }

        self.calibrator = GenericCalibration(self.settings.boards)

    def captureCount(self):
        return {
            NaoCamProps.TOP: len(self.capture_data[NaoCamProps.TOP]),
            NaoCamProps.BOTTOM: len(self.capture_data[NaoCamProps.BOTTOM])
        }

    def updateConfiguration(self, config):
        for camera in NaoCamProps.CAMERAS:
            ext = config[NaoCalibrationResult.EXT_NAMES[camera]]
            int_cc = config[NaoCalibrationResult.INT_CC_NAME[camera]]
            int_fc = config[NaoCalibrationResult.INT_FC_NAME[camera]]
            self.camerasProperties[camera].setIntrinsicScaled(int_fc, int_cc)
            self.camerasProperties[camera].setExtrinsic(ext)

    def clearCapturedData(self):
        self.capture_data[NaoCamProps.TOP] = []
        self.capture_data[NaoCamProps.BOTTOM] = []

    def captureDataToJsonFile(self, fileName):
        with open(str(fileName), 'w') as outfile:
            json.dump(
                self.capture_data,
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

            if capture_data.camera == NaoCamProps.TOP:
                self.capture_data[NaoCamProps.TOP].append(capture_data)
            elif capture_data.camera == NaoCamProps.BOTTOM:
                self.capture_data[NaoCamProps.BOTTOM].append(capture_data)
            else:
                raise ValueError("Capture data got incorrect camera name")
        return len(self.capture_data[NaoCamProps.TOP]), len(self.capture_data[NaoCamProps.BOTTOM])

    @staticmethod
    def getGroundToCamera(camera: NaoCamProps.CAMERAS, kinematicCap: NaoKinematicMatrixCapture, extTransform=np.eye(4)):
        if camera not in NaoCamProps.CAMERAS:
            raise ValueError("Camera name invalid, provided: " +
                             str(camera)+" expected one of: " + str(NaoCamProps.CAMERAS))
        groundToCameraUncalib = NaoCamProps.CAM_TO_HEAD_UNCALIB_INV[
            camera] @ kinematicCap.torso_to_head @ kinematicCap.ground_to_torso

        return Transforms.transformHomography(extTransform, groundToCameraUncalib)

    def projectBoardPoints(self,
                           capture_data: NaoCaptureData,
                           camera: NaoCamProps,
                           ext_params=[None]):
        '''
        Convenience function. Not to be used in high frequencies
        returns an array of type: (id, detected_pt, proj_pt)
        '''

        fc, cc = camera.getIntrinsic()

        ground2Cam = np.eye(4)

        if (np.array(ext_params).ravel() != None).any():
            extTransform = camera.getExtrinsicMat(ext_params)
            ground2Cam = self.getGroundToCamera(
                camera.camera, capture_data.kinematic_data, extTransform)
        else:
            ground2Cam = capture_data.kinematic_data.ground_to_camera

        twoDeepts, successes = NaoCamProps.robotToPixel(
            fc, cc, ground2Cam, capture_data.board_points_3D)

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
            logger.error(__name__ +
                         ": Exception while extracting calibration points" + str(e))

    # Consider on static or not
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
            raise ValueError("Not enough parameters supplied")

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

                # we need inverse of extrinsic matrix = transpose
                extrinsic = NaoCamProps.getExtrinsicMat(ext_param).transpose()
                proj_points = NaoCamProps.robotToPixel(
                    int_param[0], int_param[1], extrinsic,
                    boardCorners_wrt_cam_u[:, begin_offset:end_offset])

                # TODO  Do the proj_points part in a more elegant manner
                np.subtract(detectedCorners[begin_offset_2x:end_offset_2x],
                            proj_points[0].transpose().ravel().tolist()[0],
                            residual[begin_offset_2x:end_offset_2x, 0])
        if len(residual) != len(detectedCorners):
            raise ValueError("Residual cols and input cols doesnt match",
                             residual.shape, detectedCorners.shape)
        return residual.ravel()

    def verifyValues(self):
        if (len(self.capture_data[NaoCamProps.TOP]) + len(self.capture_data[NaoCamProps.BOTTOM])) <= 0:
            return False
        return True

    def startCalibration(self):

        logger.info(__name__ +
                    ": Starting Calibration preperations.")

        startTime = time.perf_counter()

        # TODO Implement this
        if not self.verifyValues():
            logger.warning(__name__ + "No captures are there.")
        result = NaoCalibrationResult()

        #### Intrinsic Calibration ####

        if self.settings.is_intrinsic[NaoCamProps.TOP] or \
                self.settings.is_intrinsic[NaoCamProps.BOTTOM]:
            # TODO Move the aruco part to calibration.py
            logger.info(__name__ +
                        ":Starting Intrinsic")
            # We only bother with the first board :P
            board_idx = next((i for i, board in enumerate(self.settings.boards)
                              if board.pattern_type ==
                              BoardProperties.PatternType.CHARUCO_BOARD), -1)

            if board_idx < 0:
                raise ValueError(
                    "A suitable board for intrinsic calib. wasn't found")
            # TODO properly handle this later
            if len(self.settings.boards) > 1:
                raise NotImplementedError(
                    "Multiple boards not supported for intrinsic.")

            for camera in NaoCamProps.CAMERAS:
                cameraProperties = self.camerasProperties[camera]
                if self.settings.is_intrinsic[camera]:

                    corners_of_all_frames = []
                    ids_of_all_frames = []

                    im_width = cameraProperties.image_width
                    im_height = cameraProperties.image_height

                    for v in self.capture_data[camera]:
                        if not v.points_per_board[board_idx]:
                            continue
                        ids = v.marker_ids[board_idx]
                        detected_pts = v.detected_points[board_idx]
                        ids_of_all_frames.append(ids)
                        corners_of_all_frames.append(detected_pts)

                    if len(ids_of_all_frames):
                        intrinsic_matrix = cameraProperties.getIntrinsicMat()
                        dist_coeffs = np.array([0, 0, 0, 0], dtype=float)

                        # ChAruco calibration
                        output = ar.calibrateCameraCharucoExtended(
                            corners_of_all_frames, ids_of_all_frames,
                            self.settings.boards[board_idx].board,
                            (im_width, im_height), intrinsic_matrix, dist_coeffs)

                        logger.debug(__name__ + ": " + str(camera) + " intrinsic -> retval, matrix, dist. coeffs" + str(output[0]) + " "
                                    + str(output[1]) + str(dist_coeffs))  # output[2]

                        result.is_int_done[camera] = True
                        fc, cc = NaoCamProps.digestIntrinsicMat(output[1])
                        fc, cc = NaoCamProps.getIntrinsicScaled(
                            fc, cc, im_width, im_height)
                        if camera == NaoCamProps.TOP:
                            result.setTopIntrinsics(fc, cc)
                        else:
                            result.setBottomIntrinsics(fc, cc)

            logger.info(__name__+": Intrinsic Calibration time-> " +
                        str((time.perf_counter() - startTime) * 1000) + "ms")

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

            for idx, cameraName in enumerate([NaoCamProps.TOP, NaoCamProps.BOTTOM]):
                # If ext top
                if self.settings.is_extrinsic[cameraName]:
                    cameraProperties = self.camerasProperties[cameraName]

                    tuning_params += cameraProperties.ext

                    offset = 0
                    sumPoints = 0

                    for v in self.capture_data[cameraName]:
                        ground2camgen = NaoCamProps.CAM_TO_HEAD_UNCALIB_INV[
                            cameraName] @ v.kinematic_data.torso_to_head @ v.kinematic_data.ground_to_torso
                        val_wrt_cam_u = Transforms.transformHomography(
                            ground2camgen, v.board_points_3D)
                        independants = np.concatenate(
                            (independants, val_wrt_cam_u), axis=1)

                        for elem in v.detected_points:
                            measurements = np.append(
                                measurements, elem.ravel())

                        offset += val_wrt_cam_u.shape[1]
                        tpts_sum += sum(v.points_per_board)

                    if cameraName == NaoCamProps.TOP:
                        intrinsic_params[idx] = cameraProperties.getIntrinsic()
                        top_offset = offset
                        tpts_sum = sumPoints
                    elif cameraName == NaoCamProps.BOTTOM:
                        intrinsic_params[idx] = cameraProperties.getIntrinsic()
                        bottom_offset = offset + top_offset
                        bpts_sum = sumPoints
                    else:
                        raise ValueError(
                            "We only support TOP or BOTTOM cameras, not " + cameraName)

            # settings for cost func.
            settings = [
                self.settings.is_extrinsic[NaoCamProps.TOP],
                self.settings.is_extrinsic[NaoCamProps.BOTTOM], top_offset, bottom_offset,
                self.settings.is_torso_calib
            ]
            logger.debug(__name__+": Calib settings and initial params" + str(settings)
                         + " " + str(tuning_params))

            # Refine settings and tuning params
            settings, tuning_params = result.encodeExtrinsicCalibparams(
                settings, tuning_params)

            if len(tuning_params) > 0:
                # Finally, invoke the solver
                logger.debug(__name__+": Start solving")

                optim_output = leastsq(
                    NaoCalibration.extrinsicCostFunc,
                    tuning_params,
                    args=(independants, measurements,
                          intrinsic_params, settings)
                )

                result.decodeExtrinsicCalibParams(settings,
                                                  list(optim_output[0]))
            else:
                logger.debug(
                    __name__+": No suitable points, extrinsic skipped")

            logger.info(__name__+": Extrinsic Calibration time-> " +
                        str((time.perf_counter() - startTime) * 1000) + "ms")

        return result
