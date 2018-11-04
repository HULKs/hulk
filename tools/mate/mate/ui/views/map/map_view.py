import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg
import PyQt5.QtWidgets as qtw

from mate.ui.views.map.model import MapModel
from mate.ui.views.map.model import LayerType
from mate.ui.views.map.map_painter import Painter
import mate.net.nao as nao


class MapView(qtw.QWidget):
    def __init__(self, map_model: MapModel, nao: nao.Nao):
        super(MapView, self).__init__()

        self.nao = nao
        self.map_model = map_model
        self.layer_painter = []
        self.pixel_viewport = qtc.QRect(0, 0, 0, 0)
        self.meter_viewport = qtc.QRectF(
            0,
            0,
            self.map_model.viewport[0],
            self.map_model.viewport[1])
        self.timer = qtc.QTimer()
        self.timer.timeout.connect(self.update)

        self.create_painter()

    def connect(self, nao: nao.Nao):
        self.nao = nao

        for layer in self.layer_painter:
            layer.connect(self.nao)

    def create_painter(self):
        for layer in self.map_model.layer:
            if layer["enabled"]:
                self.layer_painter.append(LayerType[layer["type"]][1](
                    layer,
                    self.nao))
        self.timer.start(1000 / self.map_model.fps)

    def destroy_painter(self):
        for layer in self.layer_painter:
            layer.destroy()
        self.layer_painter = []
        self.timer.stop()

    def calc_max_pixel_viewport(self):
        aspect_ratio = self.map_model.viewport[0] / self.map_model.viewport[1]

        if self.height() * aspect_ratio > self.width():
            self.pixel_viewport = qtc.QRect(
                0,
                0,
                self.width(),
                self.width() / aspect_ratio)
        else:
            self.pixel_viewport = qtc.QRect(
                0,
                0,
                self.height() * aspect_ratio,
                self.height())

    def paintEvent(self, event):
        self.calc_max_pixel_viewport()
        self.meter_viewport = qtc.QRectF(
            0,
            0,
            self.map_model.viewport[0],
            self.map_model.viewport[1])

        # Create transformation matrix to automatically rescale all field
        # coordinates (meters) to pixel coordinates and to flip the y-axis.
        transformation = qtg.QTransform()
        pixels_per_meter = (self.pixel_viewport.width() /
                            self.meter_viewport.width())
        transformation.scale(pixels_per_meter, -pixels_per_meter)

        # From here on, directly use the field coordinates (in meters).

        for layer_painter in self.layer_painter:
            painter = Painter(pixels_per_meter)
            painter.begin(self)
            painter.setTransform(transformation)
            # Translate coordinate system to field center.
            painter.translate(layer_painter.layer["settings"]["center_x"],
                              layer_painter.layer["settings"]["center_y"])

            layer_painter.paint(painter)
            painter.end()
