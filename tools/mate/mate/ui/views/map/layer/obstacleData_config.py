import PyQt5.QtWidgets as qtw
import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg

import uuid

from mate.ui.views.map.layer.obstacleData_config_view \
    import Ui_ObstacleDataConfig
from mate.ui.views.map.layer.layer_config import LayerConfig, LayerConfigMeta
import mate.net.utils as net_utils
import mate.ui.utils as ui_utils
import mate.net.nao as nao


class ObstacleDataConfig(qtw.QWidget, LayerConfig, metaclass=LayerConfigMeta):
    def __init__(self, layer, parent, update_callback, nao):
        super(ObstacleDataConfig, self).__init__(parent)

        self.layer = layer
        self.update_callback = update_callback
        self.nao = nao
        self.identifier = uuid.uuid4()
        self.ui = Ui_ObstacleDataConfig()
        self.ui.setupUi(self)

        if self.layer["settings"] is None:
            self.layer["settings"] = {
                "center_x": 5.2,
                "center_y": -3.7,
                "transformation": {
                    "key": "Brain.RobotPosition",
                    "key_lambda": 'output = input["pose"]'
                },
                "obstacles": {
                    "key": "Brain.TeamObstacleData",
                    "key_lambda": 'output = input["obstacles"]',
                    "goalPostColor": "#000000",
                    "unknownColor": "#ff5500",
                    "anonymousRobotColor": "#ff00ff",
                    "hostileRobotColor": "#ff0000",
                    "teamRobotColor": "#0000ff",
                    "fallenAnonymousRobotColor": "#7f007f",
                    "fallenHostileRobotColor": "#7f0000",
                    "fallenTeamRobotColor": "#00007f",
                    "ballColor": "#000000",
                    "freeKickAreaColor": "#ffffff",
                    "penWidth": 0.03
                }
            }
        self.settings_to_ui = {
            "center_x": (
                lambda: self.ui.spin_center_x.value(),
                lambda value: self.ui.spin_center_x.setValue(value)),
            "center_y": (
                lambda: self.ui.spin_center_y.value(),
                lambda value: self.ui.spin_center_y.setValue(value)),
            "transformation": (
                lambda:  {
                    "key": self.ui.cbx_TransformationKey.currentText(),
                    "key_lambda":
                        self.ui.edit_TransformationKeyLambda.toPlainText()
                },
                lambda settings: [
                    self.ui.cbx_TransformationKey.setCurrentText(
                        settings["key"]),
                    self.ui.edit_TransformationKeyLambda.setPlainText(
                        settings["key_lambda"])
                ]
            ),
            "obstacles": (
                lambda:  {
                    "key": self.ui.cbx_ObstacleDataKey.currentText(),
                    "key_lambda":
                        self.ui.edit_ObstacleDataKeyLambda.toPlainText(),
                    "goalPostColor":
                        self.ui.edit_goalPostColor.text(),
                    "unknownColor":
                        self.ui.edit_unknownColor.text(),
                    "anonymousRobotColor":
                        self.ui.edit_anonymousRobotColor.text(),
                    "hostileRobotColor":
                        self.ui.edit_hostileRobotColor.text(),
                    "teamRobotColor":
                        self.ui.edit_teamRobotColor.text(),
                    "fallenAnonymousRobotColor":
                        self.ui.edit_fallenAnonymousRobotColor.text(),
                    "fallenHostileRobotColor":
                        self.ui.edit_fallenHostileRobotColor.text(),
                    "fallenTeamRobotColor":
                        self.ui.edit_fallenTeamRobotColor.text(),
                    "ballColor":
                        self.ui.edit_ballColor.text(),
                    "freeKickAreaColor":
                        self.ui.edit_freeKickAreaColor.text(),
                    "penWidth":
                        self.ui.spin_penWidth.value()},
                lambda settings: [
                    self.ui.cbx_ObstacleDataKey.setCurrentText(
                        settings["key"]),
                    self.ui.edit_ObstacleDataKeyLambda.setPlainText(
                        settings["key_lambda"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_goalPostColor,
                        settings["goalPostColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_unknownColor,
                        settings["unknownColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_anonymousRobotColor,
                        settings["anonymousRobotColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_hostileRobotColor,
                        settings["hostileRobotColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_teamRobotColor,
                        settings["teamRobotColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_fallenAnonymousRobotColor,
                        settings["fallenAnonymousRobotColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_fallenHostileRobotColor,
                        settings["fallenHostileRobotColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_fallenTeamRobotColor,
                        settings["fallenTeamRobotColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_ballColor,
                        settings["ballColor"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_freeKickAreaColor,
                        settings["freeKickAreaColor"]),
                    self.ui.spin_penWidth.setValue(
                        settings["penWidth"])]
            )
        }
        self.ui.cbx_TransformationKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.ui.cbx_TransformationKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.ui.cbx_ObstacleDataKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.ui.cbx_ObstacleDataKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        ui_utils.init_Color_UI(
            self.ui.btn_goalPostColor,
            self.ui.edit_goalPostColor)
        ui_utils.init_Color_UI(
            self.ui.btn_unknownColor,
            self.ui.edit_unknownColor)
        ui_utils.init_Color_UI(
            self.ui.btn_anonymousRobotColor,
            self.ui.edit_anonymousRobotColor)
        ui_utils.init_Color_UI(
            self.ui.btn_hostileRobotColor,
            self.ui.edit_hostileRobotColor)
        ui_utils.init_Color_UI(
            self.ui.btn_teamRobotColor,
            self.ui.edit_teamRobotColor)
        ui_utils.init_Color_UI(
            self.ui.btn_fallenAnonymousRobotColor,
            self.ui.edit_fallenAnonymousRobotColor)
        ui_utils.init_Color_UI(
            self.ui.btn_fallenHostileRobotColor,
            self.ui.edit_fallenHostileRobotColor)
        ui_utils.init_Color_UI(
            self.ui.btn_fallenTeamRobotColor,
            self.ui.edit_fallenTeamRobotColor)
        ui_utils.init_Color_UI(
            self.ui.btn_ballColor,
            self.ui.edit_ballColor)
        ui_utils.init_Color_UI(
            self.ui.btn_freeKickAreaColor,
            self.ui.edit_freeKickAreaColor)
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

    def fill_cbx(self):
        ui_utils.init_cbx(
            self.ui.cbx_TransformationKey,
            self.layer["settings"]["transformation"]["key"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.ui.cbx_ObstacleDataKey,
            self.layer["settings"]["obstacles"]["key"],
            self.nao.debug_data)

    def reset_widgets(self):
        self.ui.nameObstacleDataEdit.setText(self.layer["name"])
        self.ui.enabledCheckBox.setChecked(self.layer["enabled"])
        for key in self.layer["settings"]:
            self.settings_to_ui[key][1](self.layer["settings"][key])

    def accept(self):
        self.layer["name"] = self.ui.nameObstacleDataEdit.text()
        self.layer["enabled"] = self.ui.enabledCheckBox.isChecked()
        for key in self.layer["settings"]:
            self.layer["settings"][key] = self.settings_to_ui[key][0]()
        self.update_callback()

    def discard(self):
        self.reset_widgets()
