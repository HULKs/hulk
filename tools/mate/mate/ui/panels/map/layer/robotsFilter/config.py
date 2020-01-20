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
            "robotsFilter": (
                lambda:  {
                    "key": self.cbx_Key.currentText()},
                lambda config: [
                    self.cbx_Key.setCurrentText(config["key"])])
            }

        self.cbx_Key.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbx_Key.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)

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
        self.nameLineEdit.setText(self.layer_model["name"])
        self.enabledCheckBox.setChecked(self.layer_model["enabled"])

        for key in self.layer_model["config"]:
            self.config_to_ui[key][1](self.layer_model["config"][key])

    def fill_cbx(self):
        ui_utils.init_cbx(
            self.cbx_Key,
            self.layer_model["config"]["robotsFilter"]["key"],
            self.nao.debug_data)

    def accept(self):
        self.layer_model["name"] = self.nameLineEdit.text()
        self.layer_model["enabled"] = self.enabledCheckBox.isChecked()

        for key in self.layer_model["config"]:
            self.layer_model["config"][key] = self.config_to_ui[key][0]()

        self.update_callback(self.layer_model)

    def discard(self):
        self.reset_widgets()
