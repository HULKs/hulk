import PyQt5.QtWidgets as qtw
import PyQt5.QtCore as qtc
from mate.ui.panels.map.layer._layer_config import _LayerConfig
from mate.net.nao import Nao
import mate.net.utils as net_utils
import mate.ui.utils as ui_utils
import uuid
import os


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
            "teamPlayers": (
                lambda:  {
                    "key":
                        self.cbx_TeamPlayersKey.currentText(),
                    "keyLambda":
                        self.edit_TeamPlayersKeyLambda.toPlainText(),
                    "showPlayer":
                        self.playerCheckbox.isChecked(),
                    "poseCircleDiameter":
                        self.spin_poseCircleDiameter.value(),
                    "showTarget":
                        self.targetCheckbox.isChecked(),
                    "targetCircleDiameter":
                        self.spin_targetCircleDiameter.value(),
                    "showFOV":
                        self.fovCheckbox.isChecked(),
                    "maxDistance":
                        self.spin_maxDistance.value(),
                    "cameraOpeningAngle":
                        self.spin_cameraOpeningAngle.value(),
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
                        self.edit_replacementKeeperColor.text(),
                    "showSearchPosition":
                        self.searchPositionCheckbox.isChecked(),
                    "searchPositionDiameter":
                        self.spin_searchPositionDiameter.value()},
                lambda config: [
                    self.cbx_TeamPlayersKey.setCurrentText(
                        config["key"]),
                    self.edit_TeamPlayersKeyLambda.setPlainText(
                        config["keyLambda"]),
                    self.playerCheckbox.setChecked(
                        config["showPlayer"]),
                    self.spin_poseCircleDiameter.setValue(
                        config["poseCircleDiameter"]),
                    self.targetCheckbox.setChecked(
                        config["showTarget"]),
                    self.spin_targetCircleDiameter.setValue(
                        config["targetCircleDiameter"]),
                    self.fovCheckbox.setChecked(
                        config["showFOV"]),
                    self.spin_maxDistance.setValue(
                        config["maxDistance"]),
                    self.spin_cameraOpeningAngle.setValue(
                        config["cameraOpeningAngle"]),
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
                        config["replacement_keeperColor"]),
                    self.searchPositionCheckbox.setChecked(
                        config["showSearchPosition"]),
                    self.spin_searchPositionDiameter.setValue(
                        config["searchPositionDiameter"])])
            }
        self.cbx_TeamPlayersKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.cbx_TeamPlayersKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.btnAccept.pressed.connect(self.accept)
        self.btnDiscard.pressed.connect(self.discard)
        ui_utils.init_Color_UI(
            self.btn_defaultColor,
            self.edit_defaultColor)
        ui_utils.init_Color_UI(
            self.btn_keeperColor,
            self.edit_keeperColor)
        ui_utils.init_Color_UI(
            self.btn_defenderLeftColor,
            self.edit_defenderLeftColor)
        ui_utils.init_Color_UI(
            self.btn_defenderRightColor,
            self.edit_defenderRightColor)
        ui_utils.init_Color_UI(
            self.btn_supporterColor,
            self.edit_supporterColor)
        ui_utils.init_Color_UI(
            self.btn_strikerColor,
            self.edit_strikerColor)
        ui_utils.init_Color_UI(
            self.btn_bishopColor,
            self.edit_bishopColor)
        ui_utils.init_Color_UI(
            self.btn_replacementKeeperColor,
            self.edit_replacementKeeperColor)
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
        self.nameLineEdit.setText(self.layer_model["name"])
        self.enabledCheckBox.setChecked(self.layer_model["enabled"])
        for key in self.layer_model["config"]:
            self.config_to_ui[key][1](self.layer_model["config"][key])

    def fill_cbx(self):
        ui_utils.init_cbx(
            self.cbx_TeamPlayersKey,
            self.layer_model["config"]["teamPlayers"]["key"],
            self.nao.debug_data)

    def accept(self):
        self.layer_model["name"] = self.nameLineEdit.text()
        self.layer_model["enabled"] = self.enabledCheckBox.isChecked()
        for key in self.layer_model["config"]:
            self.layer_model["config"][key] = self.config_to_ui[key][0]()
        self.update_callback(self.layer_model)

    def discard(self):
        self.reset_widgets()
