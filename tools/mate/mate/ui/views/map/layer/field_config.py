import PyQt5.QtWidgets as qtw
from mate.ui.views.map.layer.field_config_view import Ui_FieldConfig
from mate.ui.views.map.layer.layer_config import LayerConfig, LayerConfigMeta


class FieldConfig(qtw.QWidget, LayerConfig, metaclass=LayerConfigMeta):

    def __init__(self, layer, parent, update_callback, nao):

        super(FieldConfig, self).__init__(parent)

        self.layer = layer
        self.update_callback = update_callback

        self.ui = Ui_FieldConfig()
        self.ui.setupUi(self)

        if self.layer["settings"] is None:
            self.layer["settings"] = {
                "center_x": 0,
                "center_y": 0,
                "field": {
                    "width": 0,
                    "length": 0,
                    "lineWidth": 0,
                    "penaltyMarkerSize": 0,
                    "penaltyMarkerDistance": 0,
                    "penaltyAreaLength": 0,
                    "penaltyAreaWidth": 0,
                    "centerCircleDiameter": 0,
                    "borderStripWidth": 0
                },
                "goal": {
                    "postDiameter": 0,
                    "height": 0,
                    "innerWidth": 0,
                    "depth": 0
                }
            }

        self.field_types = {
            "default": {
                "center_x": 5.2,
                "center_y": -3.7,
                "field": {
                    "length": 9,
                    "width": 6,
                    "lineWidth": 0.05,
                    "penaltyMarkerSize": 0.1,
                    "penaltyMarkerDistance": 1.3,
                    "penaltyAreaLength": 0.6,
                    "penaltyAreaWidth": 2.2,
                    "centerCircleDiameter": 1.5,
                    "borderStripWidth": 0.7
                },
                "goal": {
                    "postDiameter": 0.1,
                    "height": 0.8,
                    "innerWidth": 1.5,
                    "depth": 0.5
                }
            },
            "smd": {
                "center_x": 5.2,
                "center_y": -3.7,
                "field": {
                    "length": 7.5,
                    "width": 5,
                    "lineWidth": 0.05,
                    "penaltyMarkerSize": 0.1,
                    "penaltyMarkerDistance": 1.3,
                    "penaltyAreaLength": 0.6,
                    "penaltyAreaWidth": 2.2,
                    "centerCircleDiameter": 1.25,
                    "borderStripWidth": 0.4
                },
                "goal": {
                    "postDiameter": 0.1,
                    "height": 0.8,
                    "innerWidth": 1.5,
                    "depth": 0.5
                }
            }
        }

        self.settings_to_ui = {
            "center_x": (
                lambda: self.ui.spin_center_x.value(),
                lambda value: self.ui.spin_center_x.setValue(value)),
            "center_y": (
                lambda: self.ui.spin_center_y.value(),
                lambda value: self.ui.spin_center_y.setValue(value)),
            "field": (
                lambda:  {
                    "width":
                        self.ui.spin_field_width.value(),
                    "length":
                        self.ui.spin_field_length.value(),
                    "lineWidth":
                        self.ui.spin_field_lineWidth.value(),
                    "penaltyMarkerSize":
                        self.ui.spin_field_penaltyMarkerSize.value(),
                    "penaltyMarkerDistance":
                        self.ui.spin_field_penaltyMarkerDistance.value(),
                    "penaltyAreaLength":
                        self.ui.spin_field_penaltyAreaLength.value(),
                    "penaltyAreaWidth":
                        self.ui.spin_field_penaltyAreaWidth.value(),
                    "centerCircleDiameter":
                        self.ui.spin_field_centerCircleDiameter.value(),
                    "borderStripWidth":
                        self.ui.spin_field_borderStripWidth.value()},
                lambda settings: [
                    self.ui.spin_field_width.setValue(
                        settings["width"]),
                    self.ui.spin_field_length.setValue(
                        settings["length"]),
                    self.ui.spin_field_lineWidth.setValue(
                        settings["lineWidth"]),
                    self.ui.spin_field_penaltyMarkerSize.setValue(
                        settings["penaltyMarkerSize"]),
                    self.ui.spin_field_penaltyMarkerDistance.setValue(
                        settings["penaltyMarkerDistance"]),
                    self.ui.spin_field_penaltyAreaLength.setValue(
                        settings["penaltyAreaLength"]),
                    self.ui.spin_field_penaltyAreaWidth.setValue(
                        settings["penaltyAreaWidth"]),
                    self.ui.spin_field_centerCircleDiameter.setValue(
                        settings["centerCircleDiameter"]),
                    self.ui.spin_field_borderStripWidth.setValue(
                        settings["borderStripWidth"])]),
            "goal": (
                lambda: {
                    "postDiameter": self.ui.spin_goal_postDiameter.value(),
                    "height": self.ui.spin_goal_height.value(),
                    "innerWidth": self.ui.spin_goal_innerWidth.value(),
                    "depth": self.ui.spin_goal_depth.value()},
                lambda settings: [
                    self.ui.spin_goal_postDiameter.setValue(
                        settings["postDiameter"]),
                    self.ui.spin_goal_height.setValue(settings["height"]),
                    self.ui.spin_goal_innerWidth.setValue(
                        settings["innerWidth"]),
                    self.ui.spin_goal_depth.setValue(settings["depth"])])}

        self.ui.btnAccept.pressed.connect(self.accept)
        self.ui.btnDiscard.pressed.connect(self.discard)

        self.fill_field_type_menu()
        self.reset_widgets()

    def connect(self, nao):
        pass

    def reset_widgets(self):
        self.ui.nameLineEdit.setText(self.layer["name"])
        self.ui.enabledCheckBox.setChecked(self.layer["enabled"])

        for key in self.layer["settings"]:
            self.settings_to_ui[key][1](self.layer["settings"][key])

    def fill_field_type_menu(self):
        self.ui.btnLoadFieldType.setMenu(qtw.QMenu(self.ui.btnLoadFieldType))
        for field_type in self.field_types.keys():
            self.ui.btnLoadFieldType.menu().addAction(
                field_type,
                lambda field_type=field_type: self.load_field_type(field_type))
        pass

    def load_field_type(self, field_type: str):
        self.layer["settings"
                   ]["center_x"] = self.field_types[field_type]["center_x"]
        self.layer["settings"
                   ]["center_y"] = self.field_types[field_type]["center_y"]
        self.layer["settings"]["field"] = self.field_types[field_type]["field"]
        self.layer["settings"]["goal"] = self.field_types[field_type]["goal"]
        self.reset_widgets()

    def accept(self):
        self.layer["name"] = self.ui.nameLineEdit.text()
        self.layer["enabled"] = self.ui.enabledCheckBox.isChecked()

        for key in self.layer["settings"]:
            self.layer["settings"][key] = self.settings_to_ui[key][0]()

        self.update_callback()

    def discard(self):
        self.reset_widgets()
