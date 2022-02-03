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
            "targetPosition": (
                lambda:  {
                    "key": self.cbx_targetPositionKey.currentText(),
                    "key_lambda":
                        self.edit_targetPositionKeyLambda.toPlainText(),
                    "targetCircleDiameter":
                        self.spin_targetCircleDiameter.value(),
                    "targetColor": self.edit_targetColor.text()},
                lambda config: [
                    self.cbx_targetPositionKey.setCurrentText(
                        config["key"]),
                    self.edit_targetPositionKeyLambda.setPlainText(
                        config["key_lambda"]),
                    self.spin_targetCircleDiameter.setValue(
                        config["targetCircleDiameter"]),
                    ui_utils.reset_textField_color(
                        self.edit_targetColor, config["targetColor"])]
            ),
            "displacementVector": (
                lambda:  {
                    "key": self.cbx_displacementVectorKey.currentText(),
                    "key_lambda":
                        self.edit_displacementVectorKeyLambda.toPlainText(),
                    "lineWidth":
                        self.spin_lineWidth.value(),
                    "lineColor": self.edit_lineColor.text()},
                lambda config: [
                    self.cbx_displacementVectorKey.setCurrentText(
                        config["key"]),
                    self.edit_displacementVectorKeyLambda.setPlainText(
                        config["key_lambda"]),
                    self.spin_lineWidth.setValue(
                        config["lineWidth"]),
                    ui_utils.reset_textField_color(
                        self.edit_lineColor, config["lineColor"])]
            )
        }
        self.cbx_TransformationKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.cbx_TransformationKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.cbx_targetPositionKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.cbx_targetPositionKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        ui_utils.init_Color_UI(
            self.btn_targetColor,
            self.edit_targetColor)
        self.cbx_displacementVectorKey.completer().setFilterMode(
            qtc.Qt.MatchContains)
        self.cbx_displacementVectorKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        ui_utils.init_Color_UI(
            self.btn_lineColor,
            self.edit_lineColor)
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
            self.cbx_targetPositionKey,
            self.layer_model["config"]["targetPosition"]["key"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.cbx_displacementVectorKey,
            self.layer_model["config"]["displacementVector"]["key"],
            self.nao.debug_data)

    def reset_widgets(self):
        self.nameMotionPlannerEdit.setText(self.layer_model["name"])
        self.enabledCheckBox.setChecked(self.layer_model["enabled"])
        for key in self.layer_model["config"]:
            self.config_to_ui[key][1](self.layer_model["config"][key])

    def accept(self):
        self.layer_model["name"] = self.nameMotionPlannerEdit.text()
        self.layer_model["enabled"] = self.enabledCheckBox.isChecked()
        for key in self.layer_model["config"]:
            self.layer_model["config"][key] = self.config_to_ui[key][0]()
        self.update_callback(self.layer_model)

    def discard(self):
        self.reset_widgets()
