import PyQt5.QtWidgets as qtw

import mate.ui.utils as ui_utils


class Config(qtw.QWidget):
    def __init__(self, model):
        super(Config, self).__init__()
        ui_utils.loadUi(__file__, self)

        self.model = model

        self.showGridCheckBox.stateChanged.connect(self.update_show_grid)
        self.showRigCheckBox.stateChanged.connect(self.update_show_rig)
        self.showPlotCheckBox.stateChanged.connect(self.update_show_plot)

        self.spin_grid_distance.valueChanged.connect(self.update_grid_distance)
        self.spin_bone_width.valueChanged.connect(self.update_bone_width)

        self.reset_widgets()

    def reset_widgets(self):
        self.showGridCheckBox.setChecked(self.model["showGrid"])
        self.showRigCheckBox.setChecked(self.model["showRig"])
        self.showPlotCheckBox.setChecked(self.model["showPlot"])
        self.spin_grid_distance.setValue(self.model["gridDistance"])
        self.spin_bone_width.setValue(self.model["boneWidth"])

    def update_show_grid(self):
        self.model["showGrid"] = self.showGridCheckBox.isChecked()

    def update_show_rig(self):
        self.model["showRig"] = self.showRigCheckBox.isChecked()

    def update_show_plot(self):
        self.model["showPlot"] = self.showPlotCheckBox.isChecked()

    def update_grid_distance(self):
        self.model["gridDistance"] = self.spin_grid_distance.value()

    def update_bone_width(self):
        self.model["boneWidth"] = self.spin_bone_width.value()
