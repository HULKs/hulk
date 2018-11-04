import uuid
import json
import time
from typing import Union
from datetime import datetime
from enum import Enum, IntEnum

import math
import numpy as np

from transforms3d import axangles as axangles, affines as affines

import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc
import PyQt5.QtWidgets as qtw

import mate.net.nao as nao
import mate.net.nao_data as nd
import mate.net.utils as netutils

from mate.ui.views.view.view_controller import View
from .camera_calib_view import Ui_CameraCalibration

from mate.lib.calibration.nao_calibration import NaoCamProps as n_cam, NaoCalibration as NaoCalib, NaoCalibSettings, NaoCaptureData as n_cap, NaoCalibrationResult as n_result
from mate.lib.calibration.calibration import BoardProperties as BoardProps, Transforms
from mate.lib.calibration.calib_motion_generator import CalibMotionGenerator


class ImageWithKinematicData(nd.DebugImage):
    '''
    This class will hold the individual "snapshot" sent by nao when triggered via
    calibrationCaptureTrigger config param
    TODO FUTURE -> Make to hold joint angle data
    '''
    TIMEOUT = 20  # ms

    def __init__(self, key: str, width: int, height: int, data: bytes, update: bool = False):
        super(ImageWithKinematicData, self).__init__(key, width, height, data)
        self.is_img_dat_updated = update  # make true when new data is there
        self.head_to_torso = None
        self.torso_to_ground = None
        self.camera_to_ground = None
        self.is_kinematics_updated = False
        self.cam_name = 0
        if n_cam.TOP in key:
            self.cam_name = n_cam.CamSelect.TOP
        elif n_cam.BOTTOM in key:
            self.cam_name = n_cam.CamSelect.BOTTOM

    def reset(self, image: nd.DebugImage):
        self.data = image.data
        self.width = image.width
        self.height = image.height

        self.is_img_dat_updated = True
        self.is_kinematics_updated = False

    def loadToPixMap(self, pixmap: qtg.QPixmap):
        pixmap.loadFromData(self.data)
        self.is_img_dat_updated = False

    def setKinematicChain(self, head2Torso, torso2Ground, camera2Ground):
        self.head_to_torso = head2Torso
        self.torso_to_ground = torso2Ground
        self.camera_to_ground = camera2Ground
        self.camera_to_ground[0:3, 3] *= 1000  # convert to mm
        self.is_kinematics_updated = True

    def getNaoCapData(self):
        cap_data = n_cap(self.cam_name)
        cap_data.torso_to_head = np.matrix(self.head_to_torso).I
        cap_data.ground_to_torso = np.matrix(self.torso_to_ground).I
        cap_data.camera_to_ground = np.matrix(self.camera_to_ground)
        return cap_data

    @staticmethod
    def naoDebugKinMatrixToAffine(data):
        axisAngle = data[0][:]
        tvec = data[1][:]
        return Transforms.axTransToAffine(axisAngle, tvec)

#### The actual controller. ####


class CameraCalib(qtw.QDockWidget):

    CAM_MOUNT = {
        n_cam.TOP: "Brain.CameraCalibration.top_image",
        n_cam.BOTTOM: "Brain.CameraCalibration.bottom_image"
    }
    KIN_MATRIX_MOUNTS = {
        n_cam.CAM_TO_GROUND: {
            n_cam.TOP: "Brain.CameraCalibration.Camera2Ground_top",
            n_cam.BOTTOM: "Brain.CameraCalibration.Camera2Ground_bottom",
        },
        n_cam.TORSO_TO_GROUND: "Brain.CameraCalibration.Torso2Ground",

        n_cam.HEAD_TO_TORSO: "Brain.CameraCalibration.Head2Torso"

    }
    CALIB_VISION_CFG = {"mount": "Brain.CameraCalibration",
                        "trigger_key": "calibrationCaptureTrigger"}
    CALIB_BEHAVIOR_CFG = {
        "mount": "Brain.BehaviorModule",
        "head_pitch_key": "calibrationHeadPitch",
        "head_yaw_key": "calibrationHeadYaw",
        "is_cam_calib": "isCameraCalibration"
    }

    def __init__(self,
                 nao: nao.Nao):
        super(CameraCalib, self).__init__()

        self.nao = nao
        self.identifier = uuid.uuid4()
        self.currentSubscriptions = set()

        self.setWindowTitle("Camera Calibration")
        self.ui = Ui_CameraCalibration()
        self.ui.setupUi(self)
        self.ui.configWidget.hide()
        self.ui.resultWidget.hide()
        self.ui.btnCapture.clicked.connect(self.capture)
        self.ui.btnCalibrate.clicked.connect(self.startCalib)
        self.ui.btnSave.clicked.connect(self.saveCalib)
        self.ui.btnShowResults.clicked.connect(
            lambda: self.ui.resultWidget.setVisible(not self.ui.resultWidget.isVisible()))
        self.ui.btnConfAndManualMode.clicked.connect(
            lambda: self.ui.configWidget.setVisible(not self.ui.configWidget.isVisible()))
        self.ui.checkBoxCalibModeEnable.stateChanged.connect(
            lambda: self.setCalib(self.ui.checkBoxCalibModeEnable.isChecked()))
        self.ui.btnManualMove.clicked.connect(lambda: self.moveRobot(
            float(self.ui.txtPitch.text().strip()), float(self.ui.txtYaw.text().strip())))

        # As capturing automatically project, no need to specifically call project
        self.ui.btnProjectMarkers.clicked.connect(self.capture)

        self.ui.btnPlayMotionSeq.clicked.connect(
            lambda: self.setMotionSequenceState(1))
        self.ui.btnStopMotion.clicked.connect(
            lambda: self.setMotionSequenceState(0))

        self.ui.btnDownload.clicked.connect(
            self.saveCapturesToFile)

        #### Set UI update ####
        self.frame_rate = 20
        self.updateInterval = 1.0/float(self.frame_rate)

        self.timer = qtc.QTimer()
        self.timer.timeout.connect(self.update)
        self.set_timer(self.frame_rate)

        # Motion command things.

        # The UI update timer itself will be used for timing the capture stuff.
        # motion command sending; values in seconds
        self.motionDelay = 2.0
        # wait n seconds after motion command is sent before triggering
        self.captureDelay = 2.3
        self.motionCommandList = CalibMotionGenerator.generateHeadMotion()
        self.motionSeqIndex = 0
        self.motionSeqState = 0
        self.elapsedTime = 0

        self.trigger_top = False
        self.trigger_bottom = False

        #### capture ####
        self.captureFlag = False
        # self.captureInfo = CalibCaptureInfo()

        #### store data of each capture. ####
        self.captures = []

        #### img data ####
        self.cur_top_img_data = ImageWithKinematicData(
            CameraCalib.CAM_MOUNT[n_cam.TOP], 0, 0, [], False)
        self.cur_bottom_img_data = ImageWithKinematicData(
            CameraCalib.CAM_MOUNT[n_cam.BOTTOM], 0, 0, [], False)

        self.cur_torso_to_ground = None
        self.cur_head_to_torso = None
        self.cur_cam_to_ground = {
            n_cam.TOP: None,
            n_cam.BOTTOM: None
        }

        # rotation pose for the RC17 rig (using charuco): [math.pi/2,0,-math.pi/2]
        # translation for the RC17 rig: 395 in x;
        self.calib_board = BoardProps(35, 50, 6, 11, BoardProps.PatternType.CHARUCO_BOARD, [
                                      math.pi/2, 0, -math.pi/2], [400, 150, 15])
        self.calibrator = NaoCalib(NaoCalibSettings(
            [self.calib_board], self.ui.checkBoxIntrinsicTop.isChecked(), self.ui.checkBoxIntrinsicBottom.isChecked(
            ), self.ui.checkBoxExtrinsicTop.isChecked(), self.ui.checkBoxExtrinsicBottom.isChecked()))

        self.ui.btnClearCaptures.clicked.connect(
            self.calibrator.clearCapturedData)
        self.results = None

        #### connect ####
        if self.nao.is_connected():
            self.connect(self.nao, self.frame_rate)

    #### Subscriptions, connections, etc ####
    def connect(self, nao: nao.Nao, frame_rate: int = 30):
        self.nao = nao
        self.set_timer(frame_rate)

        #### subscribe ####
        self.subscribeMulti(CameraCalib.CAM_MOUNT.values())
        self.subscribe(
            CameraCalib.KIN_MATRIX_MOUNTS[n_cam.TORSO_TO_GROUND])
        self.subscribe(
            CameraCalib.KIN_MATRIX_MOUNTS[n_cam.HEAD_TO_TORSO])
        self.subscribe(
            CameraCalib.KIN_MATRIX_MOUNTS[n_cam.CAM_TO_GROUND][n_cam.TOP])
        self.subscribe(
            CameraCalib.KIN_MATRIX_MOUNTS[n_cam.CAM_TO_GROUND][n_cam.BOTTOM])

        self.trigger(n_cam.CamSelect.NONE)

    def set_timer(self, frameRate: int):
        self.timer.stop()
        if frameRate > 0 and self.nao.is_connected():
            self.updateInterval = 1.0/self.frame_rate
            self.timer.start(1000 * self.updateInterval)

    def subscribe(self, key):
        if self.nao.is_connected() and key not in self.currentSubscriptions:
            self.nao.debug_protocol.subscribe(key, self.identifier,
                                              lambda d: self.data_received(d))
            self.currentSubscriptions.add(key)

    def unsubscribe(self, key: str = "*"):
        self.unsubscribeMulti(key)

    def unsubscribeMulti(self, keys: Union[str, list, set]):
        '''
        Unsubscribe from multiple mounts
        sending wildcard "*" instead of a list will cause unsubscription from all
        '''
        if self.nao.is_connected():
            if isinstance(keys, str) and keys == "*":
                for curSubKey in self.currentSubscriptions:
                    self.nao.debug_protocol.unsubscribe(curSubKey,
                                                        self.identifier)
            else:
                # Only attempt to unsub. already subscribed keys
                keys = set(keys)
                keys.intersection_update(self.currentSubscriptions)
                for curSubKey in keys:
                    self.nao.debug_protocol.unsubscribe(curSubKey,
                                                        self.identifier)

    def subscribeMulti(self, keys):
        '''
        Subscribe for multiple mounts
        '''
        keys = set(keys)
        if self.nao.is_connected():
            for key in keys:
                self.nao.debug_protocol.subscribe(key, self.identifier,
                                                  lambda d: self.data_received(d))
            self.currentSubscriptions.update(keys)

    def drawMarkers(self, pixmap, dataList, colour: qtg.QColor = qtc.Qt.red):
        if not (dataList and len(dataList[0]) == 2):
            if dataList[0]:
                print("Data list is not in correct shape;",
                      (len(dataList), len(dataList[0])))
            return

        painter = qtg.QPainter()
        painter.begin(pixmap)
        painter.setPen(colour)

        rect = qtc.QRectF(0, 0, 4, 4)
        for dataSet in dataList:
            proj_pt = (dataSet[1][0], dataSet[1][1])
            rect = qtc.QRectF(qtc.QPointF(
                proj_pt[0], proj_pt[1]), qtc.QSizeF(4, 4))
            painter.drawText(qtc.QPointF(
                proj_pt[0]+5, proj_pt[1]+5), str(dataSet[0]))
            painter.fillRect(rect, colour)
        painter.end()

    def saveCapturesToFile(self):
        options = qtw.QFileDialog.Options()
        options |= qtw.QFileDialog.DontUseNativeDialog
        fileName, _ = qtw.QFileDialog.getSaveFileName(
            self, "QFileDialog.getSaveFileName()", "", "All Files (*);;JSON Files (*.json)", options=options)
        if fileName:
            self.calibrator.captureDataToJsonFile(fileName)

    def setMotionSequenceState(self, val=0):
        if val > 0:
            self.motionSeqState = val
            if (not self.ui.btnStopMotion.isEnabled()) or self.ui.btnPlayMotionSeq.isEnabled():
                self.ui.btnStopMotion.setEnabled(True)
                self.ui.btnPlayMotionSeq.setEnabled(False)
        else:
            self.motionSeqState = 0
            self.motionSeqIndex = 0
            self.elapsedTime = 0
            if self.ui.btnStopMotion.isEnabled() or (not self.ui.btnPlayMotionSeq.isEnabled()):
                self.ui.btnStopMotion.setEnabled(False)
                self.ui.btnPlayMotionSeq.setEnabled(True)

    def setCalib(self, val: bool= False):
        self.setConfig(
            CameraCalib.CALIB_BEHAVIOR_CFG["mount"], CameraCalib.CALIB_BEHAVIOR_CFG["is_cam_calib"], bool(val))

    def moveRobot(self, pitch: float, yaw: float,  torso_rotV: list = [], torso_posV: list=[]):
        self.setConfig(
            CameraCalib.CALIB_BEHAVIOR_CFG["mount"], "useEffectiveYawVelocity", False)
        self.setConfig(
            CameraCalib.CALIB_BEHAVIOR_CFG["mount"], CameraCalib.CALIB_BEHAVIOR_CFG["head_yaw_key"], yaw)
        self.setConfig(
            CameraCalib.CALIB_BEHAVIOR_CFG["mount"], CameraCalib.CALIB_BEHAVIOR_CFG["head_pitch_key"], pitch)

        # TODO FUTURE set torso movements

    def trigger(self, val: n_cam.CamSelect =n_cam.CamSelect.BOTH):
        if val == n_cam.CamSelect.BOTH:
            self.trigger_top = True
            self.trigger_bottom = True
        elif val == n_cam.CamSelect.TOP:
            self.trigger_top = True
            self.trigger_bottom = False
        elif val == n_cam.CamSelect.BOTTOM:
            self.trigger_top = False
            self.trigger_bottom = True
        else:
            self.trigger_top = False
            self.trigger_bottom = False

    def capture(self):
        '''
        Take a snapshot.
        1. Image
        2. Kinematic Matrices
            torso2gnd
            head2torso
            cam2gnd
        3. FUTURE - Joint angles
        '''
        self.captureFlag = True

        # rising edge
        self.trigger(n_cam.CamSelect.BOTH)

    def startCalib(self):
        self.ui.btnCalibrate.setEnabled(False)
        self.ui.resultWidget.hide()
        self.ui.btnShowResults.setEnabled(False)

        self.ui.checkSaveTopInt.setChecked(False)
        self.ui.checkSaveTopExt.setChecked(False)
        self.ui.checkSaveBotInt.setChecked(False)
        self.ui.checkSaveBotExt.setChecked(False)

        self.ui.checkSaveTopInt.setEnabled(False)
        self.ui.checkSaveTopExt.setEnabled(False)
        self.ui.checkSaveBotInt.setEnabled(False)
        self.ui.checkSaveBotExt.setEnabled(False)
        self.ui.btnSave.setEnabled(False)

        self.calibrator.settings.setFlags(self.ui.checkBoxIntrinsicTop.isChecked(), self.ui.checkBoxIntrinsicBottom.isChecked(
        ), self.ui.checkBoxExtrinsicTop.isChecked(), self.ui.checkBoxExtrinsicBottom.isChecked())

        output = self.calibrator.startCalibration()
        if output:
            prettyOut = {}
            if output.is_ext_top_done:
                self.ui.checkSaveTopExt.setEnabled(True)
                self.ui.btnSave.setEnabled(True)
                prettyOut[n_result.TOP_EXT] = output.top_ext
            if output.is_ext_bottom_done:
                self.ui.checkSaveBotExt.setEnabled(True)
                self.ui.btnSave.setEnabled(True)
                prettyOut[n_result.BOTTOM_EXT] = output.bottom_ext
            if output.is_int_top_done:
                self.ui.checkSaveTopInt.setEnabled(True)
                self.ui.btnSave.setEnabled(True)
                prettyOut[n_result.TOP_FC] = output.top_fc
                prettyOut[n_result.TOP_CC] = output.top_cc
            if output.is_int_bottom_done:
                self.ui.checkSaveBotInt.setEnabled(True)
                self.ui.btnSave.setEnabled(True)
                prettyOut[n_result.BOTTOM_FC] = output.bottom_fc
                prettyOut[n_result.BOTTOM_CC] = output.bottom_cc

            self.ui.btnShowResults.setEnabled(True)

            self.results = output

            self.ui.txtResult.setPlainText(json.dumps(prettyOut, indent=2))
            print(json.dumps(prettyOut, indent=2))
            self.updateStatusLabel("Calibration Complete")
        else:
            self.updateStatusLabel("Calibration Failed")

        self.ui.btnCalibrate.setEnabled(True)

    def saveCalib(self):
        output = self.results
        if output:
            if output.is_ext_top_done and self.ui.checkSaveTopExt.isChecked():
                self.setConfig(
                    n_result.MOUNT, n_result.TOP_EXT, output.top_ext)
                self.calibrator.top_camera.setExtrinsic(output.top_ext)
            if output.is_ext_bottom_done and self.ui.checkSaveBotExt.isChecked():
                self.setConfig(
                    n_result.MOUNT, n_result.BOTTOM_EXT, output.bottom_ext)
                self.calibrator.bottom_camera.setExtrinsic(output.bottom_ext)
            if output.is_int_top_done and self.ui.checkSaveTopInt.isChecked():
                self.setConfig(
                    n_result.MOUNT, n_result.TOP_FC, output.top_fc)
                self.setConfig(
                    n_result.MOUNT, n_result.TOP_CC, output.top_cc)
                self.calibrator.top_camera.setIntrinsicScaled(
                    output.top_fc,  output.top_cc)
            if output.is_int_bottom_done and self.ui.checkSaveBotInt.isChecked():
                self.setConfig(
                    n_result.MOUNT, n_result.BOTTOM_FC, output.bottom_fc)
                self.setConfig(
                    n_result.MOUNT, n_result.BOTTOM_CC, output.bottom_cc)
                self.calibrator.bottom_camera.setIntrinsicScaled(
                    output.bottom_fc,  output.bottom_cc)

            self.saveConfig()
            self.updateStatusLabel("Saved Results")

    def updateStatusLabel(self, string: str):
        self.ui.lblStatus.setText(string)

    def data_received(self, data: netutils.Data):
        if isinstance(data, nd.DebugValue):
            if data.key == CameraCalib.KIN_MATRIX_MOUNTS[n_cam.TORSO_TO_GROUND]:
                self.cur_torso_to_ground = ImageWithKinematicData.naoDebugKinMatrixToAffine(
                    data.data)
            elif data.key == CameraCalib.KIN_MATRIX_MOUNTS[n_cam.HEAD_TO_TORSO]:
                self.cur_head_to_torso = ImageWithKinematicData.naoDebugKinMatrixToAffine(
                    data.data)
            elif data.key == CameraCalib.KIN_MATRIX_MOUNTS[n_cam.CAM_TO_GROUND][n_cam.TOP]:
                self.cur_cam_to_ground[n_cam.TOP] = ImageWithKinematicData.naoDebugKinMatrixToAffine(
                    data.data)
            elif data.key == CameraCalib.KIN_MATRIX_MOUNTS[n_cam.CAM_TO_GROUND][n_cam.BOTTOM]:
                self.cur_cam_to_ground[n_cam.BOTTOM] = ImageWithKinematicData.naoDebugKinMatrixToAffine(
                    data.data)

        if isinstance(data, nd.DebugImage):
            if data.key == CameraCalib.CAM_MOUNT[n_cam.TOP] and self.trigger_top:
                self.cur_top_img_data.reset(data)
                self.cur_top_img_data.setKinematicChain(
                    self.cur_head_to_torso, self.cur_torso_to_ground, self.cur_cam_to_ground[n_cam.TOP])
                self.trigger_top = False
            elif data.key == CameraCalib.CAM_MOUNT[n_cam.BOTTOM] and self.trigger_bottom:
                self.cur_bottom_img_data.reset(data)
                self.cur_bottom_img_data.setKinematicChain(
                    self.cur_head_to_torso, self.cur_torso_to_ground, self.cur_cam_to_ground[n_cam.BOTTOM])
                self.trigger_bottom = False

    def update(self):
        '''
        Update UI and update capture array if new data is there.
        '''
        # Motion sequence playing

        if self.motionSeqState:
            if self.motionSeqState == 1:
                self.trigger(n_cam.CamSelect.BOTH)
                self.setMotionSequenceState(2)
                self.elapsedTime = 0

            elif self.motionSeqState == 2 and self.elapsedTime > self.motionDelay:
                # We got the images, now the next movement
                if self.motionSeqIndex < len(self.motionCommandList):
                    idx = self.motionSeqIndex
                    command = self.motionCommandList[idx]
                    self.moveRobot(command['pitch'], command['yaw'])
                    self.setMotionSequenceState(3)
                    self.motionSeqIndex += 1
                else:
                    self.setMotionSequenceState(0)
                    self.updateStatusLabel("Capture Sequence Done")

            elif self.motionSeqState == 3 and \
                    self.elapsedTime > (self.motionDelay + self.captureDelay):
                # Movement should be done by now, start the next cycle
                self.setMotionSequenceState(1)
                self.trigger(n_cam.CamSelect.NONE)

            self.elapsedTime += self.updateInterval

        # Image updates

        # Due to preprocessing times, the timer has to be stopped until processing is done.
        # The idea is similar to stopping interrupts in an interrupt routine.
        isStopTimer = False

        if self.cur_top_img_data.is_img_dat_updated:
            startTime = time.perf_counter()

            pixmap = qtg.QPixmap()
            self.cur_top_img_data.loadToPixMap(pixmap)

            if self.cur_top_img_data.is_kinematics_updated:
                # stop timer
                if not isStopTimer:
                    self.timer.stop()
                isStopTimer = True

                count = self.calibrator.processCapture(
                    self.cur_top_img_data.getNaoCapData(), self.cur_top_img_data.data)

                self.updateStatusLabel(
                    "Snapshots captured top: "+str(count[0])+", bottom: "+str(count[1]))

                if self.calibrator.top_data:
                    cap_data = self.calibrator.top_data[-1]

                    projectedBoardPoints = self.calibrator.projectBoardPoints(
                        cap_data, self.calibrator.top_camera, Transforms.kinematicInv(cap_data.camera_to_ground))
                    self.drawMarkers(
                        pixmap, projectedBoardPoints, qtc.Qt.yellow)

                    if self.results and self.results.top_ext:

                        projectedBoardPoints = self.calibrator.projectBoardPoints(
                            cap_data, self.calibrator.top_camera,  self.calibrator.getGroundToCamera(cap_data, self.results.top_ext))

                        self.drawMarkers(
                            pixmap, projectedBoardPoints, qtc.Qt.green)

            w = self.ui.canvasTopCam.width()
            h = self.ui.canvasTopCam.height()

            self.ui.canvasTopCam.setMinimumSize(1, 1)
            self.ui.canvasTopCam.setPixmap(
                pixmap.scaled(w, h, qtc.Qt.KeepAspectRatio))

        if self.cur_bottom_img_data.is_img_dat_updated:
            startTime = time.perf_counter()

            pixmap = qtg.QPixmap()
            self.cur_bottom_img_data.loadToPixMap(pixmap)

            if self.cur_bottom_img_data.is_kinematics_updated:
                # stop timer
                if not isStopTimer:
                    self.timer.stop()
                isStopTimer = True

                count = self.calibrator.processCapture(
                    self.cur_bottom_img_data.getNaoCapData(), self.cur_bottom_img_data.data)
                self.updateStatusLabel(
                    "Snapshots captured top: "+str(count[0])+", bottom: "+str(count[1]))

                # Draw markers
                if self.calibrator.bottom_data:
                    cap_data = self.calibrator.bottom_data[-1]
                    projectedBoardPoints = self.calibrator.projectBoardPoints(
                        cap_data, self.calibrator.bottom_camera, Transforms.kinematicInv(cap_data.camera_to_ground))
                    self.drawMarkers(
                        pixmap, projectedBoardPoints, qtc.Qt.yellow)

                    if self.results and self.results.bottom_ext:
                        projectedBoardPoints = self.calibrator.projectBoardPoints(
                            cap_data, self.calibrator.bottom_camera, self.calibrator.getGroundToCamera(cap_data, self.results.bottom_ext))
                        self.drawMarkers(
                            pixmap, projectedBoardPoints, qtc.Qt.green)

            w = self.ui.canvasBottomCam.width()
            h = self.ui.canvasBottomCam.height()

            self.ui.canvasBottomCam.setMinimumSize(1, 1)
            self.ui.canvasBottomCam.setPixmap(
                pixmap.scaled(w, h, qtc.Qt.KeepAspectRatio))
        # Restart the timer if stopped
        if isStopTimer:
            self.timer.start(self.frame_rate)

    #### Config related ####

    def saveConfig(self):
        if self.nao.is_connected():
            self.nao.config_protocol.save()

    def setConfig(self, mount: str, key: str, value: str):
        if self.nao.is_connected():
            self.nao.config_protocol.set(mount, key, value)

    #### Other events ####
    def closeEvent(self, event):
        if self.nao.is_connected():
            self.trigger(n_cam.CamSelect.BOTH)
            self.setCalib(False)
            self.unsubscribe()
        self.timer.stop()
        self.deleteLater()
        super(CameraCalib, self).closeEvent(event)
