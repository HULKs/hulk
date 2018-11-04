import PyQt5.QtWidgets as qtw
import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg

import uuid

from mate.ui.views.map.layer.selfPlayer_config_view import Ui_SelfPlayerConfig
from mate.ui.views.map.layer.layer_config import LayerConfig, LayerConfigMeta
import mate.net.nao as nao
import mate.net.utils as net_utils
import mate.ui.utils as ui_utils


class SelfPlayerConfig(qtw.QWidget, LayerConfig, metaclass=LayerConfigMeta):
    def __init__(self, layer, parent, update_callback, nao: nao.Nao):
        super(SelfPlayerConfig, self).__init__(parent)
        self.nao = nao
        self.layer = layer
        self.update_callback = update_callback
        self.identifier = uuid.uuid4()
        self.ui = Ui_SelfPlayerConfig()
        self.ui.setupUi(self)
        if self.layer["settings"] is None:
            self.layer["settings"] = {
                "center_x": 5.2,
                "center_y": -3.7,
                "pose": {
                    "key": "Brain.RobotPosition",
                    "keyLambda": 'output = input["pose"]',
                    "drawPose": True,
                    "positionCircleDiameter": 0.28,
                    "orientationLineLength": 0.28,
                    "drawPlayerNumber": True,
                    "fixedColor": "#ffffff",
                    "useFixedColor": False,
                    "roleKey": "Brain.PlayingRoles",
                    "roleKeyLambda": 'output = input["role"]',
                    "defaultColor": "#ffffff",
                    "keeperColor": "#0000ff",
                    "defenderColor": "#00ff00",
                    "supporterColor": "#ff00ff",
                    "strikerColor": "#ff0000",
                    "bishopColor": "#ffff00",
                    "replacement_keeperColor": "#00ffff"
                },
                "fov": {
                    "jointSensorDataKey": "Motion.JointSensorData",
                    "jointSensorDataKeyLambda": 'output = input["angles"]',
                    "drawFOV": True,
                    "maxDistance": 2.5,
                    "cameraOpeningAngle": 60.97
                },
                "motionPlan": {
                    "key": "Brain.MotionPlanner",
                    "keyLambda": "output = input",
                    "drawMotionPlan": True,
                    "targetCircleDiameter": 0.28,
                    "drawTranslation": False,
                    "translationColor": "#ff0000"
                },
                "ballSearch": {
                    "key": "Brain.BallSearchPosition",
                    "keyLambda": 'output = input["searchPosition"]',
                    "drawSearchTarget": False,
                    "searchCircleDiameter": 0.60
                }
            }
        self.settings_to_ui = {
            "center_x": (
                lambda: self.ui.spin_center_x.value(),
                lambda value: self.ui.spin_center_x.setValue(value)),
            "center_y": (
                lambda: self.ui.spin_center_y.value(),
                lambda value: self.ui.spin_center_y.setValue(value)),
            "pose": (
                lambda:  {
                    "key":
                        self.ui.cbx_PoseKey.currentText(),
                    "keyLambda":
                        self.ui.edit_poseKeyLambda.toPlainText(),
                    "drawPose":
                        self.ui.drawPoseCheckBox.isChecked(),
                    "positionCircleDiameter":
                        self.ui.spin_positionCircleDiameter.value(),
                    "orientationLineLength":
                        self.ui.spin_orientationLineLength.value(),
                    "fixedColor":
                        self.ui.edit_fixedColor.text(),
                    "useFixedColor":
                        self.ui.useFixedColorCheckBox.isChecked(),
                    "roleKey":
                        self.ui.cbx_RoleKey.currentText(),
                    "roleKeyLambda":
                        self.ui.edit_roleKeyLambda.toPlainText(),
                    "defaultColor":
                        self.ui.edit_defaultColor.text(),
                    "keeperColor":
                        self.ui.edit_keeperColor.text(),
                    "defenderColor":
                        self.ui.edit_defenderColor.text(),
                    "supporterColor":
                        self.ui.edit_supporterColor.text(),
                    "strikerColor":
                        self.ui.edit_strikerColor.text(),
                    "bishopColor":
                        self.ui.edit_bishopColor.text(),
                    "replacement_keeperColor":
                        self.ui.edit_replacementKeeperColor.text()},
                lambda settings: [
                    self.ui.cbx_PoseKey.setCurrentText(
                        settings["key"]),
                    self.ui.edit_poseKeyLambda.setPlainText(
                        settings["keyLambda"]),
                    self.ui.drawPoseCheckBox.setChecked(
                        settings["drawPose"]),
                    self.ui.spin_positionCircleDiameter.setValue(
                        settings["positionCircleDiameter"]),
                    self.ui.spin_orientationLineLength.setValue(
                        settings["orientationLineLength"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_fixedColor,
                        settings["fixedColor"]),
                    self.ui.useFixedColorCheckBox.setChecked(
                        settings["useFixedColor"]),
                    self.ui.cbx_RoleKey.setCurrentText(
                        settings["roleKey"]),
                    self.ui.edit_roleKeyLambda.setPlainText(
                        settings["roleKeyLambda"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_defaultColor,
                        settings["defaultColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_keeperColor,
                        settings["keeperColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_defenderColor,
                        settings["defenderColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_supporterColor,
                        settings["supporterColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_strikerColor,
                        settings["strikerColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_bishopColor,
                        settings["bishopColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_replacementKeeperColor,
                        settings["replacement_keeperColor"])]),
            "fov": (
                lambda:  {
                    "jointSensorDataKey":
                        self.ui.cbx_JointSensorDataKey.currentText(),
                    "jointSensorDataKeyLambda":
                        self.ui.edit_jointSensorDataKeyLambda.toPlainText(),
                    "drawFOV":
                        self.ui.drawFOVCheckBox.isChecked(),
                    "maxDistance":
                        self.ui.spin_maxDistance.value(),
                    "cameraOpeningAngle":
                        self.ui.spin_cameraOpeningAngle.value()},
                lambda settings: [
                    self.ui.cbx_JointSensorDataKey.setCurrentText(
                        settings["jointSensorDataKey"]),
                    self.ui.edit_jointSensorDataKeyLambda.setPlainText(
                        settings["jointSensorDataKeyLambda"]),
                    self.ui.drawFOVCheckBox.setChecked(
                        settings["drawFOV"]),
                    self.ui.spin_maxDistance.setValue(
                        settings["maxDistance"]),
                    self.ui.spin_cameraOpeningAngle.setValue(
                        settings["cameraOpeningAngle"])]),
            "motionPlan": (
                lambda: {
                    "key":
                        self.ui.cbx_MotionPlannerKey.currentText(),
                    "keyLambda":
                        self.ui.edit_motionPlannerKeyLambda.toPlainText(),
                    "drawMotionPlan":
                        self.ui.drawMotionPlanCheckBox.isChecked(),
                    "targetCircleDiameter":
                        self.ui.spin_targetCircleDiameter.value(),
                    "drawTranslation":
                        self.ui.drawTranslationCheckBox.isChecked(),
                    "translationColor":
                        self.ui.edit_translationColor.text()},
                lambda settings: [
                    self.ui.cbx_MotionPlannerKey.setCurrentText(
                        settings["key"]),
                    self.ui.edit_motionPlannerKeyLambda.setPlainText(
                        settings["keyLambda"]),
                    self.ui.drawMotionPlanCheckBox.setChecked(
                        settings["drawMotionPlan"]),
                    self.ui.spin_targetCircleDiameter.setValue(
                        settings["targetCircleDiameter"]),
                    self.ui.drawTranslationCheckBox.setChecked(
                        settings["drawTranslation"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_translationColor,
                        settings["translationColor"])]),
            "ballSearch": (
                lambda: {
                    "key":
                        self.ui.cbx_BallSearchKey.currentText(),
                    "keyLambda":
                        self.ui.edit_ballSearchKeyLambda.toPlainText(),
                    "drawSearchTarget":
                        self.ui.drawBallSearchCheckBox.isChecked(),
                    "searchCircleDiameter":
                        self.ui.spin_searchCircleDiameter.value()},
                lambda settings: [
                    self.ui.cbx_BallSearchKey.setCurrentText(
                        settings["key"]),
                    self.ui.edit_ballSearchKeyLambda.setPlainText(
                        settings["keyLambda"]),
                    self.ui.drawBallSearchCheckBox.setChecked(
                        settings["drawSearchTarget"]),
                    self.ui.spin_searchCircleDiameter.setValue(
                        settings["searchCircleDiameter"])])}
        self.ui.cbx_PoseKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbx_PoseKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.ui.cbx_RoleKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbx_RoleKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.ui.cbx_JointSensorDataKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.ui.cbx_JointSensorDataKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.ui.cbx_MotionPlannerKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.ui.cbx_MotionPlannerKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.ui.cbx_BallSearchKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.ui.cbx_BallSearchKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        ui_utils.init_Color_UI(self.ui.btn_fixedColor,
                               self.ui.edit_fixedColor)
        ui_utils.init_Color_UI(self.ui.btn_defaultColor,
                               self.ui.edit_defaultColor)
        ui_utils.init_Color_UI(self.ui.btn_keeperColor,
                               self.ui.edit_keeperColor)
        ui_utils.init_Color_UI(self.ui.btn_defenderColor,
                               self.ui.edit_defenderColor)
        ui_utils.init_Color_UI(self.ui.btn_supporterColor,
                               self.ui.edit_supporterColor)
        ui_utils.init_Color_UI(self.ui.btn_strikerColor,
                               self.ui.edit_strikerColor)
        ui_utils.init_Color_UI(self.ui.btn_bishopColor,
                               self.ui.edit_bishopColor)
        ui_utils.init_Color_UI(self.ui.btn_translationColor,
                               self.ui.edit_translationColor)
        self.ui.btnAccept.pressed.connect(self.accept)
        self.ui.btnDiscard.pressed.connect(self.discard)
        self.reset_widgets()
        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: nao.Nao):
        self.nao = nao
        self.fill_cbx()
        self.nao.debug_protocol.subscribe_msg_type(
            net_utils.DebugMsgType.list, self.identifier, self.fill_cbx)

    def closeEvent(self, event):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe_msg_type(
                net_utils.DebugMsgType.list, self.identifier)

    def reset_widgets(self):
        self.ui.edit_name.setText(self.layer["name"])
        self.ui.enabledCheckBox.setChecked(self.layer["enabled"])
        for key in self.layer["settings"]:
            self.settings_to_ui[key][1](self.layer["settings"][key])

    def fill_cbx(self):
        ui_utils.init_cbx(self.ui.cbx_PoseKey,
                          self.layer["settings"]["pose"]["key"],
                          self.nao.debug_data)
        ui_utils.init_cbx(self.ui.cbx_RoleKey,
                          self.layer["settings"]["pose"]["roleKey"],
                          self.nao.debug_data)
        ui_utils.init_cbx(self.ui.cbx_JointSensorDataKey,
                          self.layer["settings"]["fov"]["jointSensorDataKey"],
                          self.nao.debug_data)
        ui_utils.init_cbx(self.ui.cbx_MotionPlannerKey,
                          self.layer["settings"]["motionPlan"]["key"],
                          self.nao.debug_data)
        ui_utils.init_cbx(self.ui.cbx_BallSearchKey,
                          self.layer["settings"]["ballSearch"]["key"],
                          self.nao.debug_data)

    def accept(self):
        self.layer["name"] = self.ui.edit_name.text()
        self.layer["enabled"] = self.ui.enabledCheckBox.isChecked()
        for key in self.layer["settings"]:
            self.layer["settings"][key] = self.settings_to_ui[key][0]()
        self.update_callback()

    def discard(self):
        self.reset_widgets()
