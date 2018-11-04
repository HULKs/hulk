import PyQt5.QtWidgets as qtw

from mate.ui.views.map.model import MapModel
from mate.ui.views.map.config_view import Ui_Config


class Config(qtw.QWidget):
    def __init__(self, map_model: MapModel):
        super(Config, self).__init__()

        self.map_model = map_model

        self.ui = Ui_Config()
        self.ui.setupUi(self)

        self.ui.btnAccept.pressed.connect(self.accept)
        self.ui.btnDiscard.pressed.connect(self.discard)

        self.reset_widgets()

    def reset_widgets(self):
        self.ui.spin_viewport_width.setValue(self.map_model.viewport[0])
        self.ui.spin_viewport_height.setValue(self.map_model.viewport[1])
        self.ui.spin_fps.setValue(self.map_model.fps)

    def accept(self):
        self.map_model.viewport = [
            self.ui.spin_viewport_width.value(),
            self.ui.spin_viewport_height.value()]
        self.map_model.fps = self.ui.spin_fps.value()

    def discard(self):
        self.reset_widgets()
