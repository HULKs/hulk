import time

import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg
import PyQt5.QtWidgets as qtw

from mate.ui.panels.map.map_painter import Painter
from mate.net.nao import Nao
from mate.debug.colorlog import ColorLog

logger = ColorLog()


class MapTab(qtw.QWidget):
    def __init__(self, parent, nao: Nao):
        super(MapTab, self).__init__(parent)
        self.parent = parent
        self.nao = nao
        self.isFirstPaint = True

        self.layer_painter = []
        self.pixel_viewport = qtc.QRect(0, 0, 0, 0)
        self.meter_viewport = qtc.QRectF(
            0,
            0,
            self.parent.model["config"]["viewport"][0],
            self.parent.model["config"]["viewport"][1])
        self.timer = qtc.QTimer()
        self.timer.timeout.connect(self.update)

    def connect(self, nao):
        self.nao = nao
        for layer in self.layer_painter:
            layer.connect(self.nao)

    def create_painter(self):
        self.isFirstPaint = True
        for layer_model in self.parent.model["layer"]:
            if layer_model["enabled"]:
                self.layer_painter.append(
                    self.parent.layer_modules[layer_model["type"]].Main(
                        layer_model, self.nao))
        self.timer.start(1000 / self.parent.model["config"]["fps"])

    def destroy_painter(self):
        for layer in self.layer_painter:
            layer.destroy()
        self.layer_painter = []
        self.timer.stop()

    def calc_max_pixel_viewport(self):
        viewport = self.parent.model["config"]["viewport"]
        aspect_ratio = viewport[0] / viewport[1]

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
        if self.isFirstPaint:
            logger.debug(__name__ + ": First painting of map")
            mapPaintTime = time.time()
        self.calc_max_pixel_viewport()
        self.meter_viewport = qtc.QRectF(
            0,
            0,
            self.parent.model["config"]["viewport"][0],
            self.parent.model["config"]["viewport"][1])

        # Create transformation matrix to automatically rescale all field
        # coordinates (meters) to pixel coordinates and to flip the y-axis.
        transformation = qtg.QTransform()
        pixels_per_meter = (self.pixel_viewport.width() /
                            self.meter_viewport.width())
        transformation.scale(pixels_per_meter, -pixels_per_meter)

        # From here on, directly use the field coordinates (in meters).

        for layer_painter in self.layer_painter:
            if self.isFirstPaint:
                logger.debug(__name__ + ": First painting of " +
                             layer_painter.name + " layer")
                layerPaintTime = time.time()
            try:
                painter = Painter(pixels_per_meter)
                painter.begin(self)
                painter.setTransform(transformation)
                # Translate coordinate system to field center.
                painter.translate(layer_painter.config["center_x"],
                                  layer_painter.config["center_y"])

                layer_painter.paint(painter)
                painter.end()
            except Exception as e:
                if self.isFirstPaint:
                    logger.error(__name__ + ": Exception when painting " +
                                 layer_painter.name + " layer: ")
                    logger.error(__name__ + ": " + str(e))
            if self.isFirstPaint:
                logger.debug(__name__ + ": First painting of " +
                             layer_painter.name + " layer took: " +
                             logger.timerLogStr(layerPaintTime))
        if self.isFirstPaint:
            logger.debug(__name__ + ": First painting of map took: " +
                         logger.timerLogStr(mapPaintTime))
            self.isFirstPaint = False
