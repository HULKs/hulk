from mate.debug.colorlog import ColorLog
from mate.lib.calibration.nao_calib_captures import *
from mate.lib.calibration.calib_motion_generator import CalibMotionGenerator
from mate.lib.calibration.calibration import BoardProperties as BoardProps, Transforms
import copy
import uuid
import json
import os
import time
import typing
from typing import Union
from datetime import datetime
from enum import Enum, IntEnum

import math
import numpy as np

from transforms3d import axangles as axangles, affines as affines

import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc
import PyQt5.QtWidgets as qtw

import mate.ui.utils as ui_utils
from mate.ui.panels._panel import _Panel

import mate.net.nao as nao
import mate.net.nao_data as nd
import mate.net.utils as netutils

from mate.lib.calibration.nao_cam_props import NaoCamProps as n_cam
from mate.lib.calibration.nao_calib_captures import\
    NaoCaptureData as n_cap,\
    NaoKinematicMatrixCapture as n_kinCap,\
    ImageWithKinematicData
from mate.lib.calibration.nao_calibration import\
    NaoCalibSettings,\
    NaoCalibration as NaoCalib,\
    NaoCalibrationResult as n_result


logger = ColorLog()


class Main(_Panel):
    name = "CameraCalib"

    CAM_MOUNT = {
        n_cam.TOP: "Brain.CameraCalibration.top_image",
        n_cam.BOTTOM: "Brain.CameraCalibration.bottom_image"
    }

    KIN_MATRIX_MOUNT = "Brain.CameraCalibration.MatrixAndImageInfos"

    CALIB_VISION_CFG = {"mount": "Brain.CameraCalibration",
                        "trigger_key": "calibrationCaptureTrigger"}
    CALIB_BEHAVIOR_CFG = {
        "mount": "Brain.BehaviorModule",
        "head_pitch_key": "calibrationHeadPitch",
        "head_yaw_key": "calibrationHeadYaw",
        "is_cam_calib": "isCameraCalibration"
    }

    update_config_signal = qtc.pyqtSignal(nd.ConfigMount)

    def __init__(self, main_window, nao: nao.Nao, model: typing.Dict = None):
        super(Main, self).__init__(main_window, self.name, nao)
        ui_utils.loadUi(__file__, self)
        self.model = model
        self.currentSubscriptions = set()
        self.configWidget.hide()
        self.resultWidget.hide()
        self.btnCapture.clicked.connect(self.capture)
        self.btnCalibrate.clicked.connect(self.startCalib)
        self.btnSave.clicked.connect(self.saveCalib)
        self.btnExport.clicked.connect(self.exportCalibToFile)
        self.btnShowResults.clicked.connect(
            lambda: self.resultWidget.setVisible(not self.resultWidget.isVisible()))
        self.btnConfAndManualMode.clicked.connect(
            lambda: self.configWidget.setVisible(not self.configWidget.isVisible()))
        self.checkBoxCalibModeEnable.stateChanged.connect(
            lambda: self.setCalib(self.checkBoxCalibModeEnable.isChecked()))
        self.btnManualMove.clicked.connect(lambda: self.moveRobot(
            float(self.txtPitch.text().strip()), float(self.txtYaw.text().strip())))

        # As capturing automatically project, no need to specifically call project
        self.btnProjectMarkers.clicked.connect(self.capture)

        self.btnPlayMotionSeq.clicked.connect(
            lambda: self.setMotionSequenceState(1))
        self.btnStopMotion.clicked.connect(
            lambda: self.setMotionSequenceState(0))

        self.btnDownload.clicked.connect(
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

        self.trigger_state = {n_cam.TOP: False, n_cam.BOTTOM: False}

        #### capture ####
        self.captureFlag = False
        # self.captureInfo = CalgetGroundToCamera

        #### store data of each capture. ####
        self.captures = []

        #### img data ####
        self.cur_cam_img_data = {
            n_cam.TOP:
            ImageWithKinematicData(n_cam.TOP,
                                   Main.CAM_MOUNT[n_cam.TOP], 0, 0, [], False),
            n_cam.BOTTOM:  ImageWithKinematicData(n_cam.BOTTOM,
                                                  Main.CAM_MOUNT[n_cam.BOTTOM], 0, 0, [], False)
        }

        self.cur_matrix_cap = {
            n_cam.TOP: n_kinCap(n_cam.TOP),
            n_cam.BOTTOM: n_kinCap(n_cam.BOTTOM)
        }

        # rotation pose for the RC17 rig (using charuco): [math.pi/2,0,-math.pi/2]
        # translation for the RC17 rig: 395 in x;
        self.calib_board = BoardProps(35, 50, 6, 11, BoardProps.PatternType.CHARUCO_BOARD, [
                                      math.pi/2, 0, -math.pi/2], [400, 150, 15])
        self.calibrator = NaoCalib(NaoCalibSettings(
            [self.calib_board], self.checkBoxIntrinsicTop.isChecked(), self.checkBoxIntrinsicBottom.isChecked(
            ), self.checkBoxExtrinsicTop.isChecked(), self.checkBoxExtrinsicBottom.isChecked()))

        self.btnClearCaptures.clicked.connect(
            self.clearCaptureBtnHandler)
        self.results = None
        self.enableDisableCalibButton()
        #### connect update config signal ####
        self.currentConfig = None
        self.update_config_signal.connect(self.updateConfig)

        #### connect ####
        if self.nao.is_connected():
            self.connect(self.nao, self.frame_rate)

    def clearCaptureBtnHandler(self):
        self.calibrator.clearCapturedData()
        self.enableDisableCalibButton()
        self.updateStatusLabel("Captures cleared")

    def enableDisableCalibButton(self):
        c = self.calibrator.captureCount()
        b = c[NaoCamProps.TOP] or c[NaoCamProps.BOTTOM]
        self.btnCalibrate.setEnabled(b)
        self.btnClearCaptures.setEnabled(b)
        self.btnDownload.setEnabled(b)

    def updateConfig(self, data: nd.ConfigMount):
        if data.key == n_result.MOUNT:
            self.currentConfig = copy.deepcopy(data.data)
            self.calibrator.updateConfiguration(self.currentConfig)
            logger.info(__name__ + "Updated config")

    #### Subscriptions, connections, etc ####
    def connect(self, nao: nao.Nao, frame_rate: int = 30):
        self.nao = nao
        self.set_timer(frame_rate)

        #### subscribe ####
        self.subscribeMulti(Main.CAM_MOUNT.values())
        self.subscribe(Main.KIN_MATRIX_MOUNT)
        self.subscribeConfig(n_result.MOUNT)

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
                logger.error(__name__ + ": drawMarkers-> Data list is not in correct shape;" +
                             str(len(dataList), len(dataList[0])))
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
            if (not self.btnStopMotion.isEnabled()) or self.btnPlayMotionSeq.isEnabled():
                self.btnStopMotion.setEnabled(True)
                self.btnPlayMotionSeq.setEnabled(False)
        else:
            self.motionSeqState = 0
            self.motionSeqIndex = 0
            self.elapsedTime = 0
            if self.btnStopMotion.isEnabled() or (not self.btnPlayMotionSeq.isEnabled()):
                self.btnStopMotion.setEnabled(False)
                self.btnPlayMotionSeq.setEnabled(True)

    def setCalib(self, val: bool = False):
        self.setConfig(
            Main.CALIB_BEHAVIOR_CFG["mount"], Main.CALIB_BEHAVIOR_CFG["is_cam_calib"], bool(val))

    def moveRobot(self, pitch: float, yaw: float,  torso_rotV: list = [], torso_posV: list = []):
        self.setConfig(
            Main.CALIB_BEHAVIOR_CFG["mount"], "useEffectiveYawVelocity", False)
        self.setConfig(
            Main.CALIB_BEHAVIOR_CFG["mount"], Main.CALIB_BEHAVIOR_CFG["head_yaw_key"], yaw)
        self.setConfig(
            Main.CALIB_BEHAVIOR_CFG["mount"], Main.CALIB_BEHAVIOR_CFG["head_pitch_key"], pitch)

        # TODO FUTURE set torso movements

    def trigger(self, val: n_cam.CamSelect = n_cam.CamSelect.BOTH):
        if val == n_cam.CamSelect.BOTH:
            self.trigger_state[n_cam.TOP] = True
            self.trigger_state[n_cam.BOTTOM] = True
        elif val == n_cam.CamSelect.TOP:
            self.trigger_state[n_cam.TOP] = True
            self.trigger_state[n_cam.BOTTOM] = False
        elif val == n_cam.CamSelect.BOTTOM:
            self.trigger_state[n_cam.TOP] = False
            self.trigger_state[n_cam.BOTTOM] = True
        else:
            self.trigger_state[n_cam.TOP] = False
            self.trigger_state[n_cam.BOTTOM] = False

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
        if not self.currentConfig:
            msg = qtw.QMessageBox()
            msg.setIcon(qtw.QMessageBox.Warning)
            msg.setText(
                "Existing Projection config is not sent from NAO yet. Want to proceed?")
            msg.setWindowTitle("Config Not set")
            msg.setStandardButtons(qtw.QMessageBox.Yes | qtw.QMessageBox.No)
            msg.setDefaultButton(qtw.QMessageBox.No)
            msg.setEscapeButton(qtw.QMessageBox.No)

            if msg.exec_() == qtw.QMessageBox.No:
                self.updateStatusLabel("Calibration Cancelled")
                return

        self.calibrator.updateConfiguration(self.currentConfig)
        enableSaveAndExport = False

        self.btnCalibrate.setEnabled(False)
        self.resultWidget.hide()
        self.btnShowResults.setEnabled(False)

        self.checkSaveTopInt.setChecked(False)
        self.checkSaveTopExt.setChecked(False)
        self.checkSaveBotInt.setChecked(False)
        self.checkSaveBotExt.setChecked(False)

        self.checkSaveTopInt.setEnabled(False)
        self.checkSaveTopExt.setEnabled(False)
        self.checkSaveBotInt.setEnabled(False)
        self.checkSaveBotExt.setEnabled(False)
        self.btnSave.setEnabled(enableSaveAndExport)
        self.btnExport.setEnabled(enableSaveAndExport)

        self.calibrator.settings.setFlags(self.checkBoxIntrinsicTop.isChecked(), self.checkBoxIntrinsicBottom.isChecked(
        ), self.checkBoxExtrinsicTop.isChecked(), self.checkBoxExtrinsicBottom.isChecked())

        output = self.calibrator.startCalibration()
        if output:
            prettyOut = {}
            if output.is_ext_done[n_cam.TOP]:
                self.checkSaveTopExt.setEnabled(True)
                enableSaveAndExport = True
                prettyOut[n_result.TOP_EXT] = output.results[n_result.TOP_EXT]
            if output.is_ext_done[n_cam.BOTTOM]:
                self.checkSaveBotExt.setEnabled(True)
                enableSaveAndExport = True
                prettyOut[n_result.BOTTOM_EXT] = output.results[n_result.BOTTOM_EXT]
            if output.is_int_done[n_cam.TOP]:
                self.checkSaveTopInt.setEnabled(True)
                enableSaveAndExport = True
                prettyOut[n_result.TOP_FC] = output.results[n_result.TOP_FC]
                prettyOut[n_result.TOP_CC] = output.results[n_result.TOP_CC]
            if output.is_int_done[n_cam.BOTTOM]:
                self.checkSaveBotInt.setEnabled(True)
                enableSaveAndExport = True
                prettyOut[n_result.BOTTOM_FC] = output.results[n_result.BOTTOM_FC]
                prettyOut[n_result.BOTTOM_CC] = output.results[n_result.BOTTOM_CC]

            self.results = output
            self.btnShowResults.setEnabled(True)
            self.btnSave.setEnabled(enableSaveAndExport)
            self.btnExport.setEnabled(enableSaveAndExport)            
            self.txtResult.setPlainText(json.dumps(prettyOut, indent=2))
            self.updateStatusLabel("Calibration Complete")
        else:
            self.updateStatusLabel("Calibration Failed")

        self.btnCalibrate.setEnabled(True)

    def saveCalib(self):
        data = self.exportCalib()

        for key, value in data.items():
            self.setConfig(n_result.MOUNT, key, value)

        self.saveConfig()
        self.updateStatusLabel("Saved Results")

    def exportCalib(self):
        output = self.results
        data = {}
        if output:
            if output.is_ext_done[n_cam.TOP] and self.checkSaveTopExt.isChecked():
                data[n_result.TOP_EXT] = output.results[n_result.TOP_EXT]
            if output.is_ext_done[n_cam.BOTTOM] and self.checkSaveBotExt.isChecked():
                data[n_result.BOTTOM_EXT] = output.results[n_result.BOTTOM_EXT]
            if output.is_int_done[n_cam.TOP] and self.checkSaveTopInt.isChecked():
                data[n_result.TOP_FC] = output.results[n_result.TOP_FC]
                data[n_result.TOP_CC] = output.results[n_result.TOP_CC]
            if output.is_int_done[n_cam.BOTTOM] and self.checkSaveBotInt.isChecked():
                data[n_result.BOTTOM_FC] = output.results[n_result.BOTTOM_FC]
                data[n_result.BOTTOM_CC] = output.results[n_result.BOTTOM_CC]
        return data

    def exportCalibToFile(self):
        data = self.exportCalib()
        currentConfig = copy.deepcopy(self.currentConfig)
        currentConfig.update(data)

        if currentConfig is None:
            return

        location = qtw.QFileDialog.getSaveFileName(
            self, "Save file",
            os.getcwd() + "/../../etc/configuration/location/default/head/" +
            n_result.MOUNT + ".json")

        if location[0] == '':
            return

        try:
            with open(location[0], 'w') as f:
                json.dump(currentConfig, f, indent=4)
                f.write("\n")
        except Exception as e:
            logger.error(__name__ +
                         ": Exception while saving config to file: " +
                         str(e))
            self.window().statusBar().showMessage(str(e))

    def updateStatusLabel(self, string: str):
        self.lblStatus.setText(string)

    def data_received(self, data: netutils.Data):
        if isinstance(data, nd.DebugValue):
            if data.key == Main.KIN_MATRIX_MOUNT:
                mat = data.data
                timestamp = data.timestamp

                # get camera from idenficiation string
                camera = list(n_cam.CAM_ENUM_TO_STR_MAP.keys())[list(n_cam.CAM_ENUM_TO_STR_MAP.values())
                                                                .index(mat["imageInfos"]["identification"])]

                torso_to_head = ImageWithKinematicData.naoDebugKinMatrixToAffine(mat[n_kinCap.getDataKeyString(
                    n_kinCap.DataKey.TORSO_TO_HEAD)])
                ground_to_torso = ImageWithKinematicData.naoDebugKinMatrixToAffine(mat[n_kinCap.getDataKeyString(
                    n_kinCap.DataKey.GROUND_TO_TORSO)])
                ground_to_cam = ImageWithKinematicData.naoDebugKinMatrixToAffine(
                    mat["imageInfos"][n_kinCap.getDataKeyString(n_kinCap.DataKey.GROUND_TO_CAM)])
                ground_to_cam[0:3, 3] *= 1000  # convert to mm

                curMatCap = self.cur_matrix_cap[camera]
                curMatCap.setValues(
                    camera, timestamp, torso_to_head, ground_to_torso, ground_to_cam)
                self.cur_matrix_cap[camera] = curMatCap

                # print("l1T", mat["ImageInfos"]["timestamp"])
                if timestamp == self.cur_cam_img_data[camera].timestamp:
                    logger.info(__name__ + ": Sync mat " + str(camera))
                    self.cur_cam_img_data[camera].kinematic_chain = copy.deepcopy(
                        curMatCap)
                    self.cur_cam_img_data[camera].is_kinematics_updated = True
                    self.cur_cam_img_data[camera].isSynced = True

            else:
                logger.warning(
                    __name__ + ": Unused debug value key: " + data.key)

        if isinstance(data, nd.DebugImage):
            for camera in n_cam.CAMERAS:
                if data.key == Main.CAM_MOUNT[camera] and self.trigger_state[camera]:
                    self.trigger_state[camera] = False
                    self.cur_cam_img_data[camera].reset(data)
                    if self.cur_matrix_cap[camera].timestamp == data.timestamp:
                        self.cur_cam_img_data[camera].kinematic_chain = copy.deepcopy(
                            self.cur_matrix_cap[camera])
                        self.cur_cam_img_data[camera].is_kinematics_updated = True
                        self.cur_cam_img_data[camera].isSynced = True
                        logger.info(__name__ + ": Sync Img " + str(camera))

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

        for camera in n_cam.CAMERAS:
            curImgData = self.cur_cam_img_data[camera]
            if curImgData.is_img_dat_updated:
                # startTime = time.perf_counter()

                pixmap = qtg.QPixmap()
                curImgData.loadToPixMap(pixmap)

                if curImgData.is_kinematics_updated:
                    # stop timer
                    if not isStopTimer:
                        self.timer.stop()
                    isStopTimer = True

                    # Process and get calib feature points
                    count = self.calibrator.processCapture(
                        curImgData.getNaoCapData(), curImgData.data)
                    # Update UI labels and update button states
                    self.enableDisableCalibButton()
                    self.updateStatusLabel(
                        "Snapshots captured top: "+str(count[0])+", bottom: "+str(count[1]))

                    if self.calibrator.capture_data[camera]:

                        cap_data = self.calibrator.capture_data[camera][-1]
                        projectedBoardPoints = self.calibrator.projectBoardPoints(
                            cap_data, self.calibrator.camerasProperties[camera])

                        self.drawMarkers(
                            pixmap, projectedBoardPoints, qtc.Qt.yellow)

                        if self.results and self.results.is_ext_done[camera]:
                            projectedBoardPoints = self.calibrator.projectBoardPoints(
                                cap_data, self.calibrator.camerasProperties[camera], self.results.getExt(camera))
                            self.drawMarkers(
                                pixmap, projectedBoardPoints, qtc.Qt.green)

                if camera == n_cam.TOP:
                    self.canvasTopCam.setMinimumSize(1, 1)
                    self.canvasTopCam.setPixmap(
                        pixmap.scaled(self.canvasTopCam.width(),  self.canvasTopCam.height(), qtc.Qt.KeepAspectRatio))
                elif camera == n_cam.BOTTOM:
                    self.canvasBottomCam.setMinimumSize(1, 1)
                    self.canvasBottomCam.setPixmap(
                        pixmap.scaled(self.canvasBottomCam.width(),
                                      self.canvasBottomCam.height(), qtc.Qt.KeepAspectRatio))
        if isStopTimer:
            self.timer.start(self.frame_rate)

    #### Config related ####

    def saveConfig(self):
        if self.nao.is_connected():
            self.nao.config_protocol.save()

    def setConfig(self, mount: str, key: str, value: str):
        if self.nao.is_connected():
            self.nao.config_protocol.set(mount, key, value)

    def subscribeConfig(self, key: str, force=False):
        if self.nao.is_connected():
            self.nao.config_protocol.subscribe(
                key,
                self.objectName(),
                lambda d: self.update_config_signal.emit(d))

    #### Other events ####
    def closeEvent(self, event):
        if self.nao.is_connected():
            self.trigger(n_cam.CamSelect.BOTH)
            self.setCalib(False)
            self.unsubscribe()
        self.timer.stop()
        self.deleteLater()
        super(Main, self).closeEvent(event)
