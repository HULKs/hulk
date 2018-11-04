import PyQt5.QtWidgets as qtw

from mate.ui.views.map.layer.coordinateSystem_config_view \
    import Ui_CoordinateSystemConfig
from mate.ui.views.map.layer.layer_config import LayerConfig, LayerConfigMeta
import mate.ui.utils as ui_utils


class CoordinateSystemConfig(qtw.QWidget,
                             LayerConfig,
                             metaclass=LayerConfigMeta):
    def __init__(self, layer, parent, update_callback, nao):
        super(CoordinateSystemConfig, self).__init__(parent)
        self.layer = layer
        self.update_callback = update_callback
        self.ui = Ui_CoordinateSystemConfig()
        self.ui.setupUi(self)
        if self.layer["settings"] is None:
            self.layer["settings"] = {
                "center_x": 5.2,
                "center_y": -3.7,
                "coordinateSystem": {
                    "width": 10.4,
                    "height": 7.4,
                    "backgroundColor": "#008000",
                    "backgroundAlpha": 255,
                    "lineWidth": 0.01,
                    "stepSizeY": 1.0,
                    "stepSizeX": 1.0,
                    "lineColor": "#808080",
                    "polar": False,
                    "polarAngleStepSize": 30.0,
                    "radialStepSize": 1.0
                }
            }
        self.settings_to_ui = {
            "center_x": (
                lambda: self.ui.spin_center_x.value(),
                lambda value: self.ui.spin_center_x.setValue(value)),
            "center_y": (
                lambda: self.ui.spin_center_y.value(),
                lambda value: self.ui.spin_center_y.setValue(value)),
            "coordinateSystem": (
                lambda:  {
                    "width":
                        self.ui.spin_coordinateSystem_width.value(),
                    "height":
                        self.ui.spin_coordinateSystem_height.value(),
                    "backgroundColor":
                        self.ui.edit_bgColor.text(),
                    "backgroundAlpha":
                        self.ui.spin_coordinateSystem_bgAlpha.value(),
                    "lineWidth":
                        self.ui.spin_coordinateSystem_lineWidth.value(),
                    "stepSizeY":
                        self.ui.spin_coordinateSystem_stepSizeY.value(),
                    "stepSizeX":
                        self.ui.spin_coordinateSystem_stepSizeX.value(),
                    "lineColor":
                        self.ui.edit_lineColor.text(),
                    "polar":
                        self.ui.polarCheckBox.isChecked(),
                    "polarAngleStepSize":
                        self.ui.spin_coordinateSystem_angleStepSize.value(),
                    "radialStepSize":
                        self.ui.spin_coordinateSystem_radialStepSize.value()},
                lambda settings: [
                    self.ui.spin_coordinateSystem_width.setValue(
                        settings["width"]),
                    self.ui.spin_coordinateSystem_height.setValue(
                        settings["height"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_bgColor,
                        settings["backgroundColor"]),
                    self.ui.spin_coordinateSystem_bgAlpha.setValue(
                        settings["backgroundAlpha"]),
                    self.ui.spin_coordinateSystem_lineWidth.setValue(
                        settings["lineWidth"]),
                    self.ui.spin_coordinateSystem_stepSizeY.setValue(
                        settings["stepSizeY"]),
                    self.ui.spin_coordinateSystem_stepSizeX.setValue(
                        settings["stepSizeX"]),
                    ui_utils.reset_textField_color(
                        self.ui.edit_lineColor,
                        settings["lineColor"]),
                    self.ui.polarCheckBox.setChecked(
                        settings["polar"]),
                    self.ui.spin_coordinateSystem_angleStepSize.setValue(
                        settings["polarAngleStepSize"]),
                    self.ui.spin_coordinateSystem_radialStepSize.setValue(
                        settings["radialStepSize"])])}
        ui_utils.init_Color_UI(self.ui.btn_bgColor, self.ui.edit_bgColor)
        ui_utils.init_Color_UI(self.ui.btn_lineColor, self.ui.edit_lineColor)
        self.ui.btnAccept.pressed.connect(self.accept)
        self.ui.btnDiscard.pressed.connect(self.discard)
        self.reset_widgets()

    def connect(self, nao):
        pass

    def reset_widgets(self):
        self.ui.nameCoordinateSystemEdit.setText(self.layer["name"])
        self.ui.enabledCheckBox.setChecked(self.layer["enabled"])
        for key in self.layer["settings"]:
            self.settings_to_ui[key][1](self.layer["settings"][key])

    def accept(self):
        self.layer["name"] = self.ui.nameCoordinateSystemEdit.text()
        self.layer["enabled"] = self.ui.enabledCheckBox.isChecked()
        for key in self.layer["settings"]:
            self.layer["settings"][key] = self.settings_to_ui[key][0]()
        self.update_callback()

    def discard(self):
        self.reset_widgets()
