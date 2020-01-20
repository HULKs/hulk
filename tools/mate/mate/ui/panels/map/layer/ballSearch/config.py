import PyQt5.QtWidgets as qtw
import PyQt5.QtCore as qtc
from mate.ui.panels.map.layer._layer_config import _LayerConfig
import mate.net.utils as netutils
import mate.net.nao as nao
import uuid
import os
import mate.ui.utils as ui_utils


class Config(qtw.QWidget, _LayerConfig):

    def __init__(self, layer_model, parent, update_callback, nao):
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
            "search": (
                lambda:  {
                    "key":
                        self.cbx_MapKey.currentText(),
                    "keyLambda":
                        self.edit_MapKeyLambda.toPlainText(),
                    "showProbability":
                        self.probabilityCheckbox.isChecked(),
                    "showNumericProbability":
                        self.probabilityNumericCheckbox.isChecked(),
                    "showAge":
                        self.ageCheckbox.isChecked(),
                    "showNumericAge":
                        self.ageNumericCheckbox.isChecked(),
                    "showVoronoiSeeds":
                        self.voronoiSeedsCheckbox.isChecked()
                },
                lambda config: [
                    self.cbx_MapKey.setCurrentText(
                        config["key"]),
                    self.edit_MapKeyLambda.setPlainText(
                        config["keyLambda"]),
                    self.probabilityCheckbox.setChecked(
                        config["showProbability"]),
                    self.probabilityNumericCheckbox.setChecked(
                        config["showNumericProbability"]),
                    self.ageCheckbox.setChecked(
                        config["showAge"]),
                    self.ageNumericCheckbox.setChecked(
                        config["showNumericAge"]),
                    self.voronoiSeedsCheckbox.setChecked(
                        config["showVoronoiSeeds"])
                ]
            )
        }

        self.cbx_MapKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbx_MapKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)

        self.btnAccept.pressed.connect(self.accept)
        self.btnDiscard.pressed.connect(self.discard)

        self.reset_widgets()
        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: nao.Nao):
        self.nao = nao
        self.fill_map()
        self.nao.debug_protocol.subscribe_msg_type(
            netutils.DebugMsgType.list,
            self.identifier,
            self.fill_map)

    def closeEvent(self, event):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe_msg_type(
                netutils.DebugMsgType.list,
                self.identifier)

    def fill_map(self):
        self.cbx_MapKey.setMinimumContentsLength(1)
        self.cbx_MapKey.clear()
        search_key = self.layer_model["config"]["search"]["key"]
        if search_key not in self.nao.debug_data:
            self.cbx_MapKey.addItem(search_key)
        for key, data in self.nao.debug_data.items():
            if not data.isImage:
                self.cbx_MapKey.addItem(key)
        self.cbx_MapKey.setCurrentText(search_key)

    def reset_widgets(self):
        self.nameLineEdit.setText(self.layer_model["name"])
        self.enabledCheckBox.setChecked(self.layer_model["enabled"])

        for key in self.layer_model["config"]:
            self.config_to_ui[key][1](self.layer_model["config"][key])

    def accept(self):
        self.layer_model["name"] = self.nameLineEdit.text()
        self.layer_model["enabled"] = self.enabledCheckBox.isChecked()
        for key in self.layer_model["config"]:
            self.layer_model["config"][key] = self.config_to_ui[key][0]()
        self.update_callback(self.layer_model)

    def discard(self):
        self.reset_widgets()
