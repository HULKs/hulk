import os

import PyQt5.QtWidgets as qtw

import mate.ui.utils as ui_utils
from mate.ui.panels.map.layer._layer_config import _LayerConfig
from mate.net.nao import Nao


class Config(qtw.QWidget, _LayerConfig):
    def __init__(self, layer_model, parent, update_callback, nao: Nao):
        super(Config, self).__init__(parent)
        ui_utils.loadUi(__file__, self)

        self.layer_model = ui_utils.load_model(os.path.dirname(__file__) +
                                               "/model.json", layer_model)
        self.update_callback = update_callback

        self.config_to_ui = {
            "center_x": (
                lambda: self.spin_center_x.value(),
                lambda value: self.spin_center_x.setValue(value)),
            "center_y": (
                lambda: self.spin_center_y.value(),
                lambda value: self.spin_center_y.setValue(value)),
            "coordinateSystem": (
                lambda: {
                    "width":
                        self.spin_coordinateSystem_width.value(),
                    "height":
                        self.spin_coordinateSystem_height.value(),
                    "backgroundColor":
                        self.edit_bgColor.text(),
                    "backgroundAlpha":
                        self.spin_coordinateSystem_bgAlpha.value(),
                    "lineWidth":
                        self.spin_coordinateSystem_lineWidth.value(),
                    "stepSizeY":
                        self.spin_coordinateSystem_stepSizeY.value(),
                    "stepSizeX":
                        self.spin_coordinateSystem_stepSizeX.value(),
                    "lineColor":
                        self.edit_lineColor.text(),
                    "polar":
                        self.polarCheckBox.isChecked(),
                    "polarAngleStepSize":
                        self.spin_coordinateSystem_angleStepSize.value(),
                    "radialStepSize":
                        self.spin_coordinateSystem_radialStepSize.value()},
                lambda config: [
                    self.spin_coordinateSystem_width.setValue(
                        config["width"]),
                    self.spin_coordinateSystem_height.setValue(
                        config["height"]),
                    ui_utils.reset_textField_color(
                        self.edit_bgColor,
                        config["backgroundColor"]),
                    self.spin_coordinateSystem_bgAlpha.setValue(
                        config["backgroundAlpha"]),
                    self.spin_coordinateSystem_lineWidth.setValue(
                        config["lineWidth"]),
                    self.spin_coordinateSystem_stepSizeY.setValue(
                        config["stepSizeY"]),
                    self.spin_coordinateSystem_stepSizeX.setValue(
                        config["stepSizeX"]),
                    ui_utils.reset_textField_color(
                        self.edit_lineColor,
                        config["lineColor"]),
                    self.polarCheckBox.setChecked(
                        config["polar"]),
                    self.spin_coordinateSystem_angleStepSize.setValue(
                        config["polarAngleStepSize"]),
                    self.spin_coordinateSystem_radialStepSize.setValue(
                        config["radialStepSize"])])}
        ui_utils.init_Color_UI(self.btn_bgColor, self.edit_bgColor)
        ui_utils.init_Color_UI(self.btn_lineColor, self.edit_lineColor)
        self.btnAccept.pressed.connect(self.accept)
        self.btnDiscard.pressed.connect(self.discard)
        self.reset_widgets()

    def connect(self, nao):
        pass

    def reset_widgets(self):
        self.nameCoordinateSystemEdit.setText(self.layer_model["name"])
        self.enabledCheckBox.setChecked(self.layer_model["enabled"])
        for key in self.layer_model["config"]:
            self.config_to_ui[key][1](self.layer_model["config"][key])

    def accept(self):
        self.layer_model["name"] = self.nameCoordinateSystemEdit.text()
        self.layer_model["enabled"] = self.enabledCheckBox.isChecked()
        for key in self.layer_model["config"]:
            self.layer_model["config"][key] = self.config_to_ui[key][0]()
        self.update_callback(self.layer_model)

    def discard(self):
        self.reset_widgets()
