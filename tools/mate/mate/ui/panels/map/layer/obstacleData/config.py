import PyQt5.QtWidgets as qtw
import PyQt5.QtCore as qtc

import uuid
import os

from mate.ui.panels.map.layer._layer_config import _LayerConfig
import mate.net.utils as net_utils
import mate.ui.utils as ui_utils
from mate.net.nao import Nao


class Config(qtw.QWidget, _LayerConfig):
    def __init__(self, layer_model, parent, update_callback, nao: Nao):
        super(Config, self).__init__(parent)
        ui_utils.loadUi(__file__, self)

        self.layer_model = ui_utils.load_model(os.path.dirname(__file__) +
                                               "/model.json", layer_model)
        self.update_callback = update_callback
        self.nao = nao
        self.identifier = uuid.uuid4()

        self.config_to_ui = {
            "center_x": (
                lambda: self.spin_center_x.value(),
                lambda value: self.spin_center_x.setValue(value)),
            "center_y": (
                lambda: self.spin_center_y.value(),
                lambda value: self.spin_center_y.setValue(value)),
            "transformation": (
                lambda:  {
                    "key": self.cbx_TransformationKey.currentText(),
                    "key_lambda":
                        self.edit_TransformationKeyLambda.toPlainText()
                },
                lambda config: [
                    self.cbx_TransformationKey.setCurrentText(
                        config["key"]),
                    self.edit_TransformationKeyLambda.setPlainText(
                        config["key_lambda"])
                ]
            ),
            "obstacles": (
                lambda:  {
                    "key": self.cbx_ObstacleDataKey.currentText(),
                    "key_lambda":
                        self.edit_ObstacleDataKeyLambda.toPlainText(),
                    "goalPostColor":
                        self.edit_goalPostColor.text(),
                    "unknownColor":
                        self.edit_unknownColor.text(),
                    "anonymousRobotColor":
                        self.edit_anonymousRobotColor.text(),
                    "hostileRobotColor":
                        self.edit_hostileRobotColor.text(),
                    "teamRobotColor":
                        self.edit_teamRobotColor.text(),
                    "fallenAnonymousRobotColor":
                        self.edit_fallenAnonymousRobotColor.text(),
                    "fallenHostileRobotColor":
                        self.edit_fallenHostileRobotColor.text(),
                    "fallenTeamRobotColor":
                        self.edit_fallenTeamRobotColor.text(),
                    "ballColor":
                        self.edit_ballColor.text(),
                    "freeKickAreaColor":
                        self.edit_freeKickAreaColor.text(),
                    "penWidth":
                        self.spin_penWidth.value()},
                lambda config: [
                    self.cbx_ObstacleDataKey.setCurrentText(
                        config["key"]),
                    self.edit_ObstacleDataKeyLambda.setPlainText(
                        config["key_lambda"]),
                    ui_utils.reset_textField_color(
                        self.edit_goalPostColor,
                        config["goalPostColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_unknownColor,
                        config["unknownColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_anonymousRobotColor,
                        config["anonymousRobotColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_hostileRobotColor,
                        config["hostileRobotColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_teamRobotColor,
                        config["teamRobotColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_fallenAnonymousRobotColor,
                        config["fallenAnonymousRobotColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_fallenHostileRobotColor,
                        config["fallenHostileRobotColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_fallenTeamRobotColor,
                        config["fallenTeamRobotColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_ballColor,
                        config["ballColor"]),
                    ui_utils.reset_textField_color(
                        self.edit_freeKickAreaColor,
                        config["freeKickAreaColor"]),
                    self.spin_penWidth.setValue(
                        config["penWidth"])]
            )
        }
        self.cbx_TransformationKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.cbx_TransformationKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.cbx_ObstacleDataKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.cbx_ObstacleDataKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        ui_utils.init_Color_UI(
            self.btn_goalPostColor,
            self.edit_goalPostColor)
        ui_utils.init_Color_UI(
            self.btn_unknownColor,
            self.edit_unknownColor)
        ui_utils.init_Color_UI(
            self.btn_anonymousRobotColor,
            self.edit_anonymousRobotColor)
        ui_utils.init_Color_UI(
            self.btn_hostileRobotColor,
            self.edit_hostileRobotColor)
        ui_utils.init_Color_UI(
            self.btn_teamRobotColor,
            self.edit_teamRobotColor)
        ui_utils.init_Color_UI(
            self.btn_fallenAnonymousRobotColor,
            self.edit_fallenAnonymousRobotColor)
        ui_utils.init_Color_UI(
            self.btn_fallenHostileRobotColor,
            self.edit_fallenHostileRobotColor)
        ui_utils.init_Color_UI(
            self.btn_fallenTeamRobotColor,
            self.edit_fallenTeamRobotColor)
        ui_utils.init_Color_UI(
            self.btn_ballColor,
            self.edit_ballColor)
        ui_utils.init_Color_UI(
            self.btn_freeKickAreaColor,
            self.edit_freeKickAreaColor)
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

    def fill_cbx(self):
        ui_utils.init_cbx(
            self.cbx_TransformationKey,
            self.layer_model["config"]["transformation"]["key"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.cbx_ObstacleDataKey,
            self.layer_model["config"]["obstacles"]["key"],
            self.nao.debug_data)

    def reset_widgets(self):
        self.nameObstacleDataEdit.setText(self.layer_model["name"])
        self.enabledCheckBox.setChecked(self.layer_model["enabled"])
        for key in self.layer_model["config"]:
            self.config_to_ui[key][1](self.layer_model["config"][key])

    def accept(self):
        self.layer_model["name"] = self.nameObstacleDataEdit.text()
        self.layer_model["enabled"] = self.enabledCheckBox.isChecked()
        for key in self.layer_model["config"]:
            self.layer_model["config"][key] = self.config_to_ui[key][0]()
        self.update_callback(self.layer_model)

    def discard(self):
        self.reset_widgets()
