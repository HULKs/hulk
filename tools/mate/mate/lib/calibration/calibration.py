'''
Detect markers, etc and generate suitable points for calibration in 2D and 3D.

__author__ = "Darshana Adikari"
__copyright__ = "Copyright 2018, RobotING@TUHH / HULKs"
__license__ = ""
__version__ = "0.1"
__maintainer__ = "Darshana Adikari"
__email__ = "darshana.adikari@tuhh.de, darshanaads@gmail.com"
__status__ = "Alpha"
'''

from enum import Enum
import math
import time

import numpy as np
import glob
import cv2 as cv
import cv2.aruco as ar
import transforms3d

from mate.lib.transforms.transforms import Transforms


class BoardProperties(object):
    PatternType = Enum('PatternType',
                       'ARUCO_BOARD, CHARUCO_BOARD, CHARUCO_DIAMOND')

    def __init__(self,
                 marker_len: int,
                 square_len: int,
                 width: int = 1,
                 height: int = 1,
                 pattern_type: PatternType = PatternType.CHARUCO_BOARD,
                 rvec=[0, 0, 0],
                 tvec=[0, 0, 0]):
        self.pattern_type = pattern_type

        #### Dictionary ####
        self.dictionary = ar.getPredefinedDictionary(ar.DICT_5X5_250)

        #### Board depends on selection ####
        self.board = None

        if self.pattern_type == BoardProperties.PatternType.CHARUCO_BOARD:
            self.board = ar.CharucoBoard_create(width, height, square_len,
                                                marker_len, self.dictionary)
        elif self.pattern_type == BoardProperties.PatternType.ARUCO_BOARD:
            raise NotImplementedError("Not implemented for ARUCO_BOARD")
        else:
            raise NotImplementedError("Not implemented for OTHERS")

        # initial guess - Either measured or guessed by pose estimation
        # and yes, super private :P
        self.__position_posV = tvec
        self.__position_rotV = rvec

        # if Highest, abs(max-min) = 0. Else adjust accordingly.
        # Higher the constrains, faster convergence in general
        # This tuning margin is applied into initial guess
        # In theory, this can be fed into lev-marq (depends on implementation)

        self.pose_tune_min = [0, 0, 0, 0, 0, 0]
        self.pose_tune_max = [0, 0, 0, 0, 0, 0]

        self.pose_matrix = Transforms.getHomographyEuler(rvec, tvec)

        # Future use.
        self.id_offset = 0

    def updatePose(self, rvec, tvec):
        self.__position_posV = tvec
        self.__position_rotV = rvec
        self.pose_matrix = Transforms.getHomographyEuler(rvec, tvec)


class GenericCalibration(object):
    def __init__(self, boards: [BoardProperties]):
        super(GenericCalibration, self).__init__()
        self.boards = boards

    def calibrateIntrinsic(self, objPoints, imgPoints, imageSize,
                           calibrationCriteria):
        pass

    def _calibrateArucoBased(self):
        pass

    def _calibrateChessBoard(self):
        pass

    def calibrateExtrinsic(self, objPoints, imgPoints, imageSize,
                           calibrationCriteria):
        pass

    def extractCalibPoints(self, image):
        '''
        Supports multiple boards.
        In reality, only ONE normal board supported.
        For multiple boards, "charuco_diamond" is used
        '''

        all_detected_corners = []
        # relative to robot's coordinates
        all_board_corners = np.empty((3, 0))
        all_ids = []
        # This will be needed when optimizing for board pose

        points_per_board = [0] * len(self.boards)

        # TODO Decide on idx after  profiling against enumerate(self.boards)
        for idx, board in enumerate(self.boards):
            if board.pattern_type == BoardProperties.PatternType.CHARUCO_BOARD or\
                    board.pattern_type == BoardProperties.PatternType.CHARUCO_DIAMOND:

                gray = cv.cvtColor(image, cv.COLOR_BGR2GRAY)
                res = ar.detectMarkers(gray, board.dictionary)

                if len(res[0]) > 0:

                    res2 = ar.interpolateCornersCharuco(
                        res[0], res[1], gray, board.board)
                    if res2[1] is not None and res2[2] is not None and len(
                            res2[1]) > 3:
                        ids = np.array(res2[2]).flatten()
                        if np.amax(ids) < len(board.board.chessboardCorners):
                            # make a copy of board 3D points
                            tempBoardPoints = np.matrix(
                                board.board.chessboardCorners,
                                copy=True).transpose()

                            # we are deleting the none-detected columns using a mask
                            # id offset may come handy in future ;)
                            mask = np.zeros(
                                tempBoardPoints.shape[1], dtype=bool)
                            mask[:] = False
                            mask[ids] = True
                            transformedPoints = Transforms.transformHomography(
                                board.pose_matrix, tempBoardPoints[:, mask])

                            # add the data.
                            all_board_corners = np.concatenate(
                                (all_board_corners, transformedPoints), axis=1)
                            points_per_board[idx] = len(ids)
                            all_detected_corners.append(res2[1])
                            all_ids.append(res2[2])
                        else:
                            raise ValueError(
                                "Id cannot have higher index than chessboard corners!",
                                len(ids), len(board.board.chessboardCorners))
            else:
                raise NotImplementedError("Not implemented for OTHER patterns")

        # TODO Decide on idx after profiling against enumerate(self.boards)
        return all_detected_corners, all_board_corners, all_ids, points_per_board
