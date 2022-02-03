import os

import PyQt5.QtWidgets as qtw
from mate.ui.panels.map.layer._layer_config import _LayerConfig
import mate.ui.utils as ui_utils
from mate.net.nao import Nao


class Config(qtw.QWidget, _LayerConfig):

    def __init__(self, layer_model, parent, update_callback, nao: Nao):
        super(Config, self).__init__(parent)
        ui_utils.loadUi(__file__, self)

        self.layer_model = ui_utils.load_model(os.path.dirname(__file__) +
                                               "/model.json", layer_model)
        self.update_callback = update_callback

        self.field_types = {
            "default": {
                "center_x": 0.0,
                "center_y": 0.0,
                "field": {
                    "length": 9,
                    "width": 6,
                    "lineWidth": 0.05,
                    "penaltyMarkerSize": 0.1,
                    "penaltyMarkerDistance": 1.3,
                    "goalBoxAreaLength": 0.6,
                    "goalBoxAreaWidth": 2.2,
                    "penaltyAreaLength": 1.65,
                    "penaltyAreaWidth": 4.0,
                    "centerCircleDiameter": 1.5,
                    "borderStripWidth": 0.7,
                    "hide_background": False
                },
                "goal": {
                    "postDiameter": 0.1,
                    "height": 0.8,
                    "innerWidth": 1.5,
                    "depth": 0.5
                }
            },
            "smd": {
                "center_x": 0.0,
                "center_y": 0.0,
                "field": {
                    "length": 7.5,
                    "width": 5,
                    "lineWidth": 0.05,
                    "penaltyMarkerSize": 0.1,
                    "penaltyMarkerDistance": 1.3,
                    "goalBoxAreaLength": 0.6,
                    "goalBoxAreaWidth": 2.2,
                    "penaltyAreaLength": 1.65,
                    "penaltyAreaWidth": 3.6,
                    "centerCircleDiameter": 1.25,
                    "borderStripWidth": 0.4,
                    "hide_background": False
                },
                "goal": {
                    "postDiameter": 0.1,
                    "height": 0.8,
                    "innerWidth": 1.5,
                    "depth": 0.5
                }
            }
        }

        self.config_to_ui = {
            "center_x": (
                lambda: self.spin_center_x.value(),
                lambda value: self.spin_center_x.setValue(value)),
            "center_y": (
                lambda: self.spin_center_y.value(),
                lambda value: self.spin_center_y.setValue(value)),
            "field": (
                lambda:  {
                    "width":
                        self.spin_field_width.value(),
                    "length":
                        self.spin_field_length.value(),
                    "lineWidth":
                        self.spin_field_lineWidth.value(),
                    "penaltyMarkerSize":
                        self.spin_field_penaltyMarkerSize.value(),
                    "penaltyMarkerDistance":
                        self.spin_field_penaltyMarkerDistance.value(),
                    "goalBoxAreaLength":
                        self.spin_field_goalBoxAreaLength.value(),
                    "goalBoxAreaWidth":
                        self.spin_field_goalBoxAreaWidth.value(),
                    "penaltyAreaLength":
                        self.spin_field_penaltyAreaLength.value(),
                    "penaltyAreaWidth":
                        self.spin_field_penaltyAreaWidth.value(),
                    "centerCircleDiameter":
                        self.spin_field_centerCircleDiameter.value(),
                    "borderStripWidth":
                        self.spin_field_borderStripWidth.value(),
                    "hide_background":
                        self.hide_background_check_box.checkState()},
                lambda config: [
                    self.spin_field_width.setValue(
                        config["width"]),
                    self.spin_field_length.setValue(
                        config["length"]),
                    self.spin_field_lineWidth.setValue(
                        config["lineWidth"]),
                    self.spin_field_penaltyMarkerSize.setValue(
                        config["penaltyMarkerSize"]),
                    self.spin_field_penaltyMarkerDistance.setValue(
                        config["penaltyMarkerDistance"]),
                    self.spin_field_goalBoxAreaLength.setValue(
                        config["goalBoxAreaLength"]),
                    self.spin_field_goalBoxAreaWidth.setValue(
                        config["goalBoxAreaWidth"]),
                    self.spin_field_penaltyAreaLength.setValue(
                        config["penaltyAreaLength"]),
                    self.spin_field_penaltyAreaWidth.setValue(
                        config["penaltyAreaWidth"]),
                    self.spin_field_centerCircleDiameter.setValue(
                        config["centerCircleDiameter"]),
                    self.spin_field_borderStripWidth.setValue(
                        config["borderStripWidth"]),
                    self.hide_background_check_box.setChecked(
                        config["hide_background"])
                ]),
            "goal": (
                lambda: {
                    "postDiameter": self.spin_goal_postDiameter.value(),
                    "height": self.spin_goal_height.value(),
                    "innerWidth": self.spin_goal_innerWidth.value(),
                    "depth": self.spin_goal_depth.value()},
                lambda config: [
                    self.spin_goal_postDiameter.setValue(
                        config["postDiameter"]),
                    self.spin_goal_height.setValue(config["height"]),
                    self.spin_goal_innerWidth.setValue(
                        config["innerWidth"]),
                    self.spin_goal_depth.setValue(config["depth"])])}

        self.btnAccept.pressed.connect(self.accept)
        self.btnDiscard.pressed.connect(self.discard)

        self.fill_field_type_menu()
        self.reset_widgets()

    def connect(self, nao):
        pass

    def reset_widgets(self):
        self.nameLineEdit.setText(self.layer_model["name"])
        self.enabledCheckBox.setChecked(self.layer_model["enabled"])

        for key in self.layer_model["config"]:
            self.config_to_ui[key][1](self.layer_model["config"][key])

    def fill_field_type_menu(self):
        self.btnLoadFieldType.setMenu(qtw.QMenu(self.btnLoadFieldType))
        for field_type in self.field_types.keys():
            self.btnLoadFieldType.menu().addAction(
                field_type,
                lambda field_type=field_type: self.load_field_type(field_type))

    def load_field_type(self, field_type: str):
        for v in ["center_x", "center_y", "field", "goal"]:
            self.layer_model["config"][v] = self.field_types[field_type][v]
        self.reset_widgets()

    def accept(self):
        self.layer_model["name"] = self.nameLineEdit.text()
        self.layer_model["enabled"] = self.enabledCheckBox.isChecked()
        for key in self.layer_model["config"]:
            self.layer_model["config"][key] = self.config_to_ui[key][0]()
        self.update_callback(self.layer_model)

    def discard(self):
        self.reset_widgets()
