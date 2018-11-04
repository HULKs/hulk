import PyQt5.QtWidgets as qtw
import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg

import uuid

from mate.ui.views.map.layer.sonarSensors_config_view \
    import Ui_SonarSensorsConfig
from mate.ui.views.map.layer.layer_config import LayerConfig, LayerConfigMeta
import mate.net.utils as net_utils
import mate.ui.utils as ui_utils
import mate.net.nao as nao


class SonarSensorsConfig(qtw.QWidget, LayerConfig, metaclass=LayerConfigMeta):
    def __init__(self, layer, parent, update_callback, nao):
        super(SonarSensorsConfig, self).__init__(parent)

        self.layer = layer
        self.update_callback = update_callback
        self.nao = nao
        self.identifier = uuid.uuid4()
        self.ui = Ui_SonarSensorsConfig()
        self.ui.setupUi(self)

        if self.layer["settings"] is None:
            self.layer["settings"] = {
                "center_x": 5.2,
                "center_y": -3.7,
                "transformation": {
                    "key": "Brain.RobotPosition",
                    "key_lambda": 'output = input["pose"]'
                },
                "sonar": {
                    "rawKey": "Motion.SonarSensorData",
                    "rawKey_lambda": 'output = input',
                    "filteredKey": "Motion.SonarData",
                    "filteredKey_lambda": 'output = input',
                    "openingAngle": 60.0,
                    "zAngle": 25.0,
                    "yOffset": 4.16,
                    "color": "#ffffff"
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
            "sonar": (
                lambda:  {
                    "rawKey":
                        self.ui.cbx_RawSonarKey.currentText(),
                    "rawKey_lambda":
                        self.ui.edit_RawSonarKeyLambda.toPlainText(),
                    "filteredKey":
                        self.ui.cbx_FilteredSonarKey.currentText(),
                    "filteredKey_lambda":
                        self.ui.edit_FilteredSonarKeyLambda.toPlainText(),
                    "openingAngle":
                        self.ui.spin_sonarOpeningAngle.value(),
                    "zAngle":
                        self.ui.spin_sonarZAngle.value(),
                    "yOffset":
                        self.ui.spin_sonarYOffset.value(),
                    "color":
                        self.ui.edit_sonarColor.text()},
                lambda settings: [
                    self.ui.cbx_RawSonarKey.setCurrentText(
                        settings["rawKey"]),
                    self.ui.edit_RawSonarKeyLambda.setPlainText(
                        settings["rawKey_lambda"]),
                    self.ui.cbx_FilteredSonarKey.setCurrentText(
                        settings["filteredKey"]),
                    self.ui.edit_FilteredSonarKeyLambda.setPlainText(
                        settings["filteredKey_lambda"]),
                    self.ui.spin_sonarOpeningAngle.setValue(
                        settings["openingAngle"]),
                    self.ui.spin_sonarZAngle.setValue(
                        settings["zAngle"]),
                    self.ui.spin_sonarYOffset.setValue(
                        settings["yOffset"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_sonarColor, settings["color"])]
            )
        }
        self.ui.cbx_TransformationKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.ui.cbx_TransformationKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.ui.cbx_RawSonarKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.ui.cbx_RawSonarKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.ui.cbx_FilteredSonarKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.ui.cbx_FilteredSonarKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        ui_utils.init_Color_UI(
            self.ui.btn_sonarColor,
            self.ui.edit_sonarColor)
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
            self.ui.cbx_RawSonarKey,
            self.layer["settings"]["sonar"]["rawKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.ui.cbx_FilteredSonarKey,
            self.layer["settings"]["sonar"]["filteredKey"],
            self.nao.debug_data)

    def reset_widgets(self):
        self.ui.nameSonarEdit.setText(self.layer["name"])
        self.ui.enabledCheckBox.setChecked(self.layer["enabled"])
        for key in self.layer["settings"]:
            self.settings_to_ui[key][1](self.layer["settings"][key])

    def accept(self):
        self.layer["name"] = self.ui.nameSonarEdit.text()
        self.layer["enabled"] = self.ui.enabledCheckBox.isChecked()
        for key in self.layer["settings"]:
            self.layer["settings"][key] = self.settings_to_ui[key][0]()
        self.update_callback()

    def discard(self):
        self.reset_widgets()
