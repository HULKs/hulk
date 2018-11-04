import PyQt5.QtWidgets as qtw
import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg
from mate.ui.views.map.layer.teamPlayers_config_view \
    import Ui_TeamPlayersConfig
from mate.ui.views.map.layer.layer_config import LayerConfig
from mate.ui.views.map.layer.layer_config import LayerConfigMeta
import mate.net.nao as nao
import mate.net.utils as net_utils
import mate.ui.utils as ui_utils
import uuid


class TeamPlayersConfig(qtw.QWidget, LayerConfig, metaclass=LayerConfigMeta):
    def __init__(self, layer, parent, update_callback, nao: nao.Nao):
        super(TeamPlayersConfig, self).__init__(parent)

        self.nao = nao
        self.layer = layer
        self.update_callback = update_callback
        self.identifier = uuid.uuid4()

        self.ui = Ui_TeamPlayersConfig()
        self.ui.setupUi(self)

        if self.layer["settings"] is None:
            self.layer["settings"] = {
                "center_x": 5.2,
                "center_y": -3.7,
                "teamPlayers": {
                    "key": "Brain.TeamPlayers",
                    "keyLambda": 'output = input["players"]',
                    "showPlayer": True,                    
                    "poseCircleDiameter": 0.28,
                    "showTarget": True,
                    "targetCircleDiameter": 0.28,
                    "showFOV": False,
                    "maxDistance": 2.5,
                    "cameraOpeningAngle": 60.97,
                    "defaultColor": "#ffffff",
                    "keeperColor": "#0000ff",
                    "defenderColor": "#00ff00",
                    "supporterColor": "#ff00ff",
                    "strikerColor": "#ff0000",
                    "bishopColor": "#ffff00",
                    "replacement_keeperColor" : "#00ffff",
                    "showSearchPosition": False,
                    "searchPositionDiameter": 0.6
                }
            }

        self.settings_to_ui = {
            "center_x": (
                lambda: self.ui.spin_center_x.value(),
                lambda value: self.ui.spin_center_x.setValue(value)),
            "center_y": (
                lambda: self.ui.spin_center_y.value(),
                lambda value: self.ui.spin_center_y.setValue(value)),
            "teamPlayers": (
                lambda:  {
                    "key":
                        self.ui.cbx_TeamPlayersKey.currentText(),
                    "keyLambda":
                        self.ui.edit_TeamPlayersKeyLambda.toPlainText(),
                    "showPlayer":
                        self.ui.playerCheckbox.isChecked(),
                    "poseCircleDiameter":
                        self.ui.spin_poseCircleDiameter.value(),
                    "showTarget":
                        self.ui.targetCheckbox.isChecked(),
                    "targetCircleDiameter":
                        self.ui.spin_targetCircleDiameter.value(),
                    "showFOV":
                        self.ui.fovCheckbox.isChecked(),
                    "maxDistance":
                        self.ui.spin_maxDistance.value(),
                    "cameraOpeningAngle":
                        self.ui.spin_cameraOpeningAngle.value(),
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
                        self.ui.edit_replacementKeeperColor.text(),
                    "showSearchPosition":
                        self.ui.searchPositionCheckbox.isChecked(),
                    "searchPositionDiameter":
                        self.ui.spin_searchPositionDiameter.value()},
                lambda settings: [
                    self.ui.cbx_TeamPlayersKey.setCurrentText(
                        settings["key"]),
                    self.ui.edit_TeamPlayersKeyLambda.setPlainText(
                        settings["keyLambda"]),
                    self.ui.playerCheckbox.setChecked(
                        settings["showPlayer"]),
                    self.ui.spin_poseCircleDiameter.setValue(
                        settings["poseCircleDiameter"]),
                    self.ui.targetCheckbox.setChecked(
                        settings["showTarget"]),
                    self.ui.spin_targetCircleDiameter.setValue(
                        settings["targetCircleDiameter"]),
                    self.ui.fovCheckbox.setChecked(
                        settings["showFOV"]),
                    self.ui.spin_maxDistance.setValue(
                        settings["maxDistance"]),
                    self.ui.spin_cameraOpeningAngle.setValue(
                        settings["cameraOpeningAngle"]),
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
                        settings["replacement_keeperColor"]),
                    self.ui.searchPositionCheckbox.setChecked(
                        settings["showSearchPosition"]),
                    self.ui.spin_searchPositionDiameter.setValue(
                        settings["searchPositionDiameter"])])
            }
        self.ui.cbx_TeamPlayersKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.ui.cbx_TeamPlayersKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.ui.btnAccept.pressed.connect(self.accept)
        self.ui.btnDiscard.pressed.connect(self.discard)
        ui_utils.init_Color_UI(
            self.ui.btn_defaultColor,
            self.ui.edit_defaultColor)
        ui_utils.init_Color_UI(
            self.ui.btn_keeperColor,
            self.ui.edit_keeperColor)
        ui_utils.init_Color_UI(
            self.ui.btn_defenderColor,
            self.ui.edit_defenderColor)
        ui_utils.init_Color_UI(
            self.ui.btn_supporterColor,
            self.ui.edit_supporterColor)
        ui_utils.init_Color_UI(
            self.ui.btn_strikerColor,
            self.ui.edit_strikerColor)
        ui_utils.init_Color_UI(
            self.ui.btn_bishopColor,
            self.ui.edit_bishopColor)
        ui_utils.init_Color_UI(
            self.ui.btn_replacementKeeperColor,
            self.ui.edit_replacementKeeperColor)
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
        self.ui.nameLineEdit.setText(self.layer["name"])
        self.ui.enabledCheckBox.setChecked(self.layer["enabled"])
        for key in self.layer["settings"]:
            self.settings_to_ui[key][1](self.layer["settings"][key])

    def fill_cbx(self):
        ui_utils.init_cbx(
            self.ui.cbx_TeamPlayersKey,
            self.layer["settings"]["teamPlayers"]["key"],
            self.nao.debug_data)

    def accept(self):
        self.layer["name"] = self.ui.nameLineEdit.text()
        self.layer["enabled"] = self.ui.enabledCheckBox.isChecked()
        for key in self.layer["settings"]:
            self.layer["settings"][key] = self.settings_to_ui[key][0]()
        self.update_callback()

    def discard(self):
        self.reset_widgets()
