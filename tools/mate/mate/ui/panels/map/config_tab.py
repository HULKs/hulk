import PyQt5.QtWidgets as qtw

import mate.ui.utils as ui_utils


class ConfigTab(qtw.QWidget):
    def __init__(self, parent):
        super(ConfigTab, self).__init__(parent)
        ui_utils.loadUi(__file__, self)

        self.parent = parent

        self.btnAccept.pressed.connect(self.accept)
        self.btnDiscard.pressed.connect(self.discard)

        self.reset_widgets()

    def reset_widgets(self):
        self.spin_viewport_width.setValue(
            self.parent.model["config"]["viewport"][0])
        self.spin_viewport_height.setValue(
            self.parent.model["config"]["viewport"][1])
        self.spin_fps.setValue(self.parent.model["config"]["fps"])

        self.enable_perspective_check_box.setCheckState(
            self.parent.model["enable_perspective"])
        self.flip_x_check_box.setCheckState(self.parent.model["flip_x"])
        self.flip_y_check_box.setCheckState(self.parent.model["flip_y"])

        # for some reason this is true by default
        self.enable_perspective_check_box.setTristate(False)
        self.flip_x_check_box.setTristate(False)
        self.flip_y_check_box.setTristate(False)

    def accept(self):
        self.parent.model["config"]["viewport"] = [
            self.spin_viewport_width.value(),
            self.spin_viewport_height.value()]
        self.parent.model["config"]["fps"] = self.spin_fps.value()
        self.parent.model["enable_perspective"] = self.enable_perspective_check_box.isChecked(
        )
        self.parent.model["flip_x"] = self.flip_x_check_box.isChecked()
        self.parent.model["flip_y"] = self.flip_y_check_box.isChecked()

    def discard(self):
        self.reset_widgets()
