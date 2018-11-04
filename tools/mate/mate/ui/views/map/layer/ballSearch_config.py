import PyQt5.QtWidgets as qtw
import PyQt5.QtCore as qtc
from mate.ui.views.map.layer.ballSearch_config_view import Ui_BallSearchConfig
from mate.ui.views.map.layer.layer_config import LayerConfig, LayerConfigMeta
import mate.net.utils as netutils
import mate.net.nao as nao
import uuid


class BallSearchConfig(qtw.QWidget, LayerConfig, metaclass=LayerConfigMeta):

    def __init__(self, layer, parent, update_callback, nao):
        super(BallSearchConfig, self).__init__(parent)
        self.layer = layer
        self.update_callback = update_callback
        self.nao = nao
        self.identifier = uuid.uuid4()

        self.ui = Ui_BallSearchConfig()
        self.ui.setupUi(self)

        if self.layer["settings"] is None:
            self.layer["settings"] = {
                "center_x": 5.2,
                "center_y": -3.7,
                "search": {
                    "key": "Brain.BallSearchMap",
                    "keyLambda": "output = input",
                    "showProbability": True,
                    "showNumericProbability": False,
                    "showAge": True,
                    "showNumericAge": False,
                    "showVoronoiSeeds": True
                }
            }

        self.settings_to_ui = {
            "center_x": (
                lambda: self.ui.spin_center_x.value(),
                lambda value: self.ui.spin_center_x.setValue(value)),
            "center_y": (
                lambda: self.ui.spin_center_y.value(),
                lambda value: self.ui.spin_center_y.setValue(value)),
            "search": (
                lambda:  {
                    "key":
                        self.ui.cbx_MapKey.currentText(),
                    "keyLambda":
                        self.ui.edit_MapKeyLambda.toPlainText(),
                    "showProbability":
                        self.ui.probabilityCheckbox.isChecked(),
                    "showNumericProbability":
                        self.ui.probabilityNumericCheckbox.isChecked(),
                    "showAge":
                        self.ui.ageCheckbox.isChecked(),
                    "showNumericAge":
                        self.ui.ageNumericCheckbox.isChecked(),
                    "showVoronoiSeeds":
                        self.ui.voronoiSeedsCheckbox.isChecked()
                },
                lambda settings: [
                    self.ui.cbx_MapKey.setCurrentText(
                        settings["key"]),
                    self.ui.edit_MapKeyLambda.setPlainText(
                        settings["keyLambda"]),
                    self.ui.probabilityCheckbox.setChecked(
                        settings["showProbability"]),
                    self.ui.probabilityNumericCheckbox.setChecked(
                        settings["showNumericProbability"]),
                    self.ui.ageCheckbox.setChecked(
                        settings["showAge"]),
                    self.ui.ageNumericCheckbox.setChecked(
                        settings["showNumericAge"]),
                    self.ui.voronoiSeedsCheckbox.setChecked(
                        settings["showVoronoiSeeds"])
                ]
            )
        }

        self.ui.cbx_MapKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbx_MapKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)

        self.ui.btnAccept.pressed.connect(self.accept)
        self.ui.btnDiscard.pressed.connect(self.discard)

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
        self.ui.cbx_MapKey.setMinimumContentsLength(1)
        self.ui.cbx_MapKey.clear()
        if self.layer["settings"]["search"]["key"] not in self.nao.debug_data:
            self.ui.cbx_MapKey.addItem(
                self.layer["settings"]["search"]["key"])
        for key, data in self.nao.debug_data.items():
            if not data.isImage:
                self.ui.cbx_MapKey.addItem(key)
        self.ui.cbx_MapKey.setCurrentText(
            self.layer["settings"]["search"]["key"])

    def reset_widgets(self):
        self.ui.nameLineEdit.setText(self.layer["name"])
        self.ui.enabledCheckBox.setChecked(self.layer["enabled"])

        for key in self.layer["settings"]:
            self.settings_to_ui[key][1](self.layer["settings"][key])

    def accept(self):
        self.layer["name"] = self.ui.nameLineEdit.text()
        self.layer["enabled"] = self.ui.enabledCheckBox.isChecked()

        for key in self.layer["settings"]:
            self.layer["settings"][key] = self.settings_to_ui[key][0]()

        self.update_callback()

    def discard(self):
        self.reset_widgets()
