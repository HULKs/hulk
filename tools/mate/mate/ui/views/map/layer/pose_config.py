import PyQt5.QtWidgets as qtw
import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg
from mate.ui.views.map.layer.pose_config_view import Ui_PoseConfig
from mate.ui.views.map.layer.layer_config import LayerConfig, LayerConfigMeta
import mate.net.nao as nao
import mate.net.utils as net_utils
import mate.ui.utils as ui_utils
import uuid


class PoseConfig(qtw.QWidget, LayerConfig, metaclass=LayerConfigMeta):
    def __init__(self, layer, parent, update_callback, nao: nao.Nao):
        super(PoseConfig, self).__init__(parent)

        self.nao = nao
        self.layer = layer
        self.update_callback = update_callback
        self.identifier = uuid.uuid4()
        self.ui = Ui_PoseConfig()
        self.ui.setupUi(self)

        if self.layer["settings"] is None:
            self.layer["settings"] = {
                "center_x": 5.2,
                "center_y": -3.7,
                "transformation": {
                    "key": "None",
                    "key_lambda": "output=intput"
                },
                "pose": {
                    "key": "Brain.RobotPosition",
                    "keyLambda": 'output = input["pose"]',
                    "positionCircleDiameter": 0.35,
                    "orientationLineLength": 0.35,
                    "color": "#ffffff"
                },
                "fov": {
                    "jointSensorDataKey":
                        "Motion.JointSensorData",
                    "jointSensorDataKeyLambda": 'output = input["angles"]',
                    "maxDistance": 2.5,
                    "cameraOpeningAngle": 60.97
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
                        self.ui.edit_TransformationKeyLambda.toPlainText(),
                },
                lambda settings: [
                    self.ui.cbx_TransformationKey.setCurrentText(
                        settings["key"]),
                    self.ui.edit_TransformationKeyLambda.setPlainText(
                        settings["key_lambda"]),
                ]
            ),
            "pose": (
                lambda:  {
                    "key": self.ui.cbx_PoseKey.currentText(),
                    "keyLambda": self.ui.edit_poseKeyLambda.toPlainText(),
                    "positionCircleDiameter":
                        self.ui.spin_positionCircleDiameter.value(),
                    "orientationLineLength":
                        self.ui.spin_orientationLineLength.value(),
                    "color": self.ui.edit_poseColor.text()
                },
                lambda settings: [
                    self.ui.cbx_PoseKey.setCurrentText(settings["key"]),
                    self.ui.edit_poseKeyLambda.setPlainText(
                        settings["keyLambda"]),
                    self.ui.spin_positionCircleDiameter.setValue(
                        settings["positionCircleDiameter"]),
                    self.ui.spin_orientationLineLength.setValue(
                        settings["orientationLineLength"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_poseColor, settings["color"])
                ]),
            "fov": (
                lambda:  {
                    "jointSensorDataKey":
                        self.ui.cbx_JointSensorDataKey.currentText(),
                    "jointSensorDataKeyLambda":
                        self.ui.edit_jointSensorDataKeyLambda.toPlainText(),
                    "maxDistance":
                        self.ui.spin_maxDistance.value(),
                    "cameraOpeningAngle":
                        self.ui.spin_cameraOpeningAngle.value()},
                lambda settings: [
                    self.ui.cbx_JointSensorDataKey.setCurrentText(
                        settings["jointSensorDataKey"]),
                    self.ui.edit_jointSensorDataKeyLambda.setPlainText(
                        settings["jointSensorDataKeyLambda"]),
                    self.ui.spin_maxDistance.setValue(
                        settings["maxDistance"]),
                    self.ui.spin_cameraOpeningAngle.setValue(
                        settings["cameraOpeningAngle"])]
            )
        }
        self.ui.cbx_PoseKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.ui.cbx_PoseKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.ui.cbx_TransformationKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.ui.cbx_TransformationKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.ui.cbx_JointSensorDataKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.ui.cbx_JointSensorDataKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        ui_utils.init_Color_UI(
            self.ui.btn_poseColor,
            self.ui.edit_poseColor)
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
        self.ui.nameLineEdit.setText(self.layer["name"])
        self.ui.enabledCheckBox.setChecked(self.layer["enabled"])
        for key in self.layer["settings"]:
            self.settings_to_ui[key][1](self.layer["settings"][key])

    def fill_cbx(self):
        ui_utils.init_cbx(
            self.ui.cbx_TransformationKey,
            self.layer["settings"]["transformation"]["key"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.ui.cbx_PoseKey,
            self.layer["settings"]["pose"]["key"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.ui.cbx_JointSensorDataKey,
            self.layer["settings"]["fov"]["jointSensorDataKey"],
            self.nao.debug_data)

    def accept(self):
        self.layer["name"] = self.ui.nameLineEdit.text()
        self.layer["enabled"] = self.ui.enabledCheckBox.isChecked()
        for key in self.layer["settings"]:
            self.layer["settings"][key] = self.settings_to_ui[key][0]()
        self.update_callback()

    def discard(self):
        self.reset_widgets()
