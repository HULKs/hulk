import PyQt5.QtWidgets as qtw
import PyQt5.QtCore as qtc

import uuid
import os

from mate.ui.panels.map.layer._layer_config import _LayerConfig
from mate.net.nao import Nao
import mate.net.utils as net_utils
import mate.ui.utils as ui_utils


class Config(qtw.QWidget, _LayerConfig):
    def __init__(self, layer_model, parent, update_callback, nao: Nao):
        super(Config, self).__init__(parent)
        ui_utils.loadUi(__file__, self)

        self.nao = nao
        self.layer_model = ui_utils.load_model(os.path.dirname(__file__) +
                                               "/model.json", layer_model)
        self.update_callback = update_callback
        self.identifier = uuid.uuid4()

        self.config_to_ui = {
            "center_x": (
                lambda: self.spin_center_x.value(),
                lambda value: self.spin_center_x.setValue(value)),
            "center_y": (
                lambda: self.spin_center_y.value(),
                lambda value: self.spin_center_y.setValue(value)),
            "pose": (
                lambda:  {
                    "key":
                        self.cbx_PoseKey.currentText(),
                    "keyLambda":
                        self.edit_poseKeyLambda.toPlainText(),
                    "drawPose":
                        self.drawPoseCheckBox.isChecked(),
                    "positionCircleDiameter":
                        self.spin_positionCircleDiameter.value(),
                    "orientationLineLength":
                        self.spin_orientationLineLength.value(),
                    "fixedColor":
                        self.edit_fixedColor.text(),
                    "useFixedColor":
                        self.useFixedColorCheckBox.isChecked(),
                    "roleKey":
                        self.cbx_RoleKey.currentText(),
                    "roleKeyLambda":
                        self.edit_roleKeyLambda.toPlainText(),
                    "defaultColor":
                        self.edit_defaultColor.text(),
                    "keeperColor":
                        self.edit_keeperColor.text(),
                    "defenderLeftColor":
                        self.edit_defenderLeftColor.text(),
                    "defenderRightColor":
                        self.edit_defenderRightColor.text(),
                    "supporterColor":
                        self.edit_supporterColor.text(),
                    "strikerColor":
                        self.edit_strikerColor.text(),
                    "bishopColor":
                        self.edit_bishopColor.text(),
                    "replacement_keeperColor":
                        self.edit_replacementKeeperColor.text()},
                lambda config: [
                    self.cbx_PoseKey.setCurrentText(
                        config["key"]),
                    self.edit_poseKeyLambda.setPlainText(
                        config["keyLambda"]),
                    self.drawPoseCheckBox.setChecked(
                        config["drawPose"]),
                    self.spin_positionCircleDiameter.setValue(
                        config["positionCircleDiameter"]),
                    self.spin_orientationLineLength.setValue(
                        config["orientationLineLength"]),
                    ui_utils.reset_textField_color(
                        self.edit_fixedColor,
                        config["fixedColor"]),
                    self.useFixedColorCheckBox.setChecked(
                        config["useFixedColor"]),
                    self.cbx_RoleKey.setCurrentText(
                        config["roleKey"]),
                    self.edit_roleKeyLambda.setPlainText(
                        config["roleKeyLambda"]),
                    ui_utils.reset_textField_color(
                        self.edit_defaultColor,
                        config["defaultColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_keeperColor,
                        config["keeperColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_defenderLeftColor,
                        config["defenderLeftColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_defenderRightColor,
                        config["defenderRightColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_supporterColor,
                        config["supporterColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_strikerColor,
                        config["strikerColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_bishopColor,
                        config["bishopColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_replacementKeeperColor,
                        config["replacement_keeperColor"])]),
            "fov": (
                lambda:  {
                    "jointSensorDataKey":
                        self.cbx_JointSensorDataKey.currentText(),
                    "jointSensorDataKeyLambda":
                        self.edit_jointSensorDataKeyLambda.toPlainText(),
                    "drawFOV":
                        self.drawFOVCheckBox.isChecked(),
                    "maxDistance":
                        self.spin_maxDistance.value(),
                    "cameraOpeningAngle":
                        self.spin_cameraOpeningAngle.value()},
                lambda config: [
                    self.cbx_JointSensorDataKey.setCurrentText(
                        config["jointSensorDataKey"]),
                    self.edit_jointSensorDataKeyLambda.setPlainText(
                        config["jointSensorDataKeyLambda"]),
                    self.drawFOVCheckBox.setChecked(
                        config["drawFOV"]),
                    self.spin_maxDistance.setValue(
                        config["maxDistance"]),
                    self.spin_cameraOpeningAngle.setValue(
                        config["cameraOpeningAngle"])]),
            "motionPlan": (
                lambda: {
                    "key":
                        self.cbx_MotionPlannerKey.currentText(),
                    "keyLambda":
                        self.edit_motionPlannerKeyLambda.toPlainText(),
                    "drawMotionPlan":
                        self.drawMotionPlanCheckBox.isChecked(),
                    "targetCircleDiameter":
                        self.spin_targetCircleDiameter.value(),
                    "drawTranslation":
                        self.drawTranslationCheckBox.isChecked(),
                    "translationColor":
                        self.edit_translationColor.text()},
                lambda config: [
                    self.cbx_MotionPlannerKey.setCurrentText(
                        config["key"]),
                    self.edit_motionPlannerKeyLambda.setPlainText(
                        config["keyLambda"]),
                    self.drawMotionPlanCheckBox.setChecked(
                        config["drawMotionPlan"]),
                    self.spin_targetCircleDiameter.setValue(
                        config["targetCircleDiameter"]),
                    self.drawTranslationCheckBox.setChecked(
                        config["drawTranslation"]),
                    ui_utils.reset_textField_color(
                        self.edit_translationColor,
                        config["translationColor"])]),
            "ballSearch": (
                lambda: {
                    "key":
                        self.cbx_BallSearchKey.currentText(),
                    "keyLambda":
                        self.edit_ballSearchKeyLambda.toPlainText(),
                    "drawSearchTarget":
                        self.drawBallSearchCheckBox.isChecked(),
                    "searchCircleDiameter":
                        self.spin_searchCircleDiameter.value()},
                lambda config: [
                    self.cbx_BallSearchKey.setCurrentText(
                        config["key"]),
                    self.edit_ballSearchKeyLambda.setPlainText(
                        config["keyLambda"]),
                    self.drawBallSearchCheckBox.setChecked(
                        config["drawSearchTarget"]),
                    self.spin_searchCircleDiameter.setValue(
                        config["searchCircleDiameter"])])}
        self.cbx_PoseKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbx_PoseKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.cbx_RoleKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbx_RoleKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.cbx_JointSensorDataKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.cbx_JointSensorDataKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.cbx_MotionPlannerKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.cbx_MotionPlannerKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.cbx_BallSearchKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.cbx_BallSearchKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        ui_utils.init_Color_UI(self.btn_fixedColor,
                               self.edit_fixedColor)
        ui_utils.init_Color_UI(self.btn_defaultColor,
                               self.edit_defaultColor)
        ui_utils.init_Color_UI(self.btn_keeperColor,
                               self.edit_keeperColor)
        ui_utils.init_Color_UI(self.btn_defenderLeftColor,
                               self.edit_defenderLeftColor)
        ui_utils.init_Color_UI(self.btn_defenderRightColor,
                               self.edit_defenderRightColor)
        ui_utils.init_Color_UI(self.btn_supporterColor,
                               self.edit_supporterColor)
        ui_utils.init_Color_UI(self.btn_strikerColor,
                               self.edit_strikerColor)
        ui_utils.init_Color_UI(self.btn_bishopColor,
                               self.edit_bishopColor)
        ui_utils.init_Color_UI(self.btn_translationColor,
                               self.edit_translationColor)
        self.btnAccept.pressed.connect(self.accept)
        self.btnDiscard.pressed.connect(self.discard)
        self.reset_widgets()
        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: Nao):
        self.nao = nao
        self.fill_cbx()
        self.nao.debug_protocol.subscribe_msg_type(
            net_utils.DebugMsgType.list, self.identifier, self.fill_cbx)

    def closeEvent(self, event):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe_msg_type(
                net_utils.DebugMsgType.list, self.identifier)

    def reset_widgets(self):
        self.edit_name.setText(self.layer_model["name"])
        self.enabledCheckBox.setChecked(self.layer_model["enabled"])
        for key in self.layer_model["config"]:
            self.config_to_ui[key][1](self.layer_model["config"][key])

    def fill_cbx(self):
        ui_utils.init_cbx(self.cbx_PoseKey,
                          self.layer_model["config"]["pose"]["key"],
                          self.nao.debug_data)
        ui_utils.init_cbx(self.cbx_RoleKey,
                          self.layer_model["config"]["pose"]["roleKey"],
                          self.nao.debug_data)
        ui_utils.init_cbx(self.cbx_JointSensorDataKey,
                          self.layer_model["config"
                                           ]["fov"
                                             ]["jointSensorDataKey"],
                          self.nao.debug_data)
        ui_utils.init_cbx(self.cbx_MotionPlannerKey,
                          self.layer_model["config"]["motionPlan"]["key"],
                          self.nao.debug_data)
        ui_utils.init_cbx(self.cbx_BallSearchKey,
                          self.layer_model["config"]["ballSearch"]["key"],
                          self.nao.debug_data)

    def accept(self):
        self.layer_model["name"] = self.edit_name.text()
        self.layer_model["enabled"] = self.enabledCheckBox.isChecked()
        for key in self.layer_model["config"]:
            self.layer_model["config"][key] = self.config_to_ui[key][0]()
        self.update_callback(self.layer_model)

    def discard(self):
        self.reset_widgets()
