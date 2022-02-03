import time
import math

import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg
import PyQt5.QtWidgets as qtw

from mate.ui.panels.map.map_painter import Painter
from mate.net.nao import Nao
from mate.debug.colorlog import ColorLog

logger = ColorLog()


class MapTab(qtw.QWidget):
    def __init__(self, parent, nao: Nao, points):
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
        self.counter = 0
        self.points = points
        self.dragging = -1

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

    def mousePressEvent(self, event):
        p = event.pos().x(), event.pos().y()
        d = self.pixel_viewport.width(), self.pixel_viewport.height()
        p = [p[i] / d[i] for i in range(2)]

        self.dragging = -1
        closest_dist = 100000
        for i in range(4):
            dist = math.sqrt((p[0] - self.points[i][0]) **
                             2 + (p[1] - self.points[i][1]) ** 2)
            if dist < closest_dist:
                closest_dist = dist
                self.dragging = i

    def mouseMoveEvent(self, event):
        if self.dragging >= 0:
            p = event.pos().x(), event.pos().y()
            d = self.pixel_viewport.width(), self.pixel_viewport.height()
            p = [p[i] / d[i] for i in range(2)]
            self.points[self.dragging] = p

    def mouseReleaseEvent(self, event):
        self.dragging = -1

    def paintEvent(self, event):
        import math

        if self.isFirstPaint:
            logger.debug(__name__ + ": First painting of map")
            mapPaintTime = time.time()
        self.calc_max_pixel_viewport()
        self.meter_viewport = qtc.QRectF(
            0,
            0,
            self.parent.model["config"]["viewport"][0],
            self.parent.model["config"]["viewport"][1])

        self.counter += 1

        # Create transformation matrix to automatically rescale all field
        # coordinates (meters) to pixel coordinates and to flip the y-axis.
        pixels_per_meter = (self.pixel_viewport.width() /
                            self.meter_viewport.width())
        w = self.parent.model["config"]["viewport"][0]
        h = self.parent.model["config"]["viewport"][1]
        polyon = qtg.QPolygonF([
            qtc.QPointF(0.0, 0.0),
            qtc.QPointF(w * pixels_per_meter, 0.0),
            qtc.QPointF(w * pixels_per_meter, h * pixels_per_meter),
            qtc.QPointF(0.0, h * pixels_per_meter),
        ])
        polyon2 = qtg.QPolygonF([
            qtc.QPointF(
                p[0] * w * pixels_per_meter,
                p[1] * h * pixels_per_meter
            ) for p in self.points
        ])
        perspective = qtg.QTransform()
        qtg.QTransform.quadToQuad(polyon, polyon2, perspective)

        # From here on, directly use the field coordinates (in meters).
        fieldScale = qtg.QTransform()

        for layer_painter in self.layer_painter:
            if self.isFirstPaint:
                logger.debug(__name__ + ": First painting of " +
                             layer_painter.name + " layer")
                layerPaintTime = time.time()
            try:
                transformation = qtg.QTransform()
                transformation.scale(pixels_per_meter, -pixels_per_meter)

                layerFieldScale = getattr(layer_painter, "fieldScale", None)
                if layerFieldScale is not None:
                    fieldScale = qtg.QTransform()
                    fieldScale.scale(layerFieldScale[0], layerFieldScale[1])

                painter = Painter(pixels_per_meter)
                painter.begin(self)
                painter.resetTransform()
                if self.parent.model["enable_perspective"] and not layer_painter.ignore_perspective:
                    painter.setTransform(perspective)
                painter.setTransform(transformation, True)
                # Translate coordinate system to field center.
                painter.translate(w / 2.0 + layer_painter.config["center_x"],
                                  -h / 2.0 + layer_painter.config["center_y"])
                if layer_painter.ignore_scale:
                    painter.scale(w, h)
                if self.parent.model["enable_perspective"] and not layer_painter.ignore_perspective:
                    painter.setTransform(fieldScale, True)
                # These are swapped because in SPL cooredinates, the Y axis is the long side of the field
                if self.parent.model["flip_x"]:
                    painter.scale(1.0, -1.0)
                if self.parent.model["flip_y"]:
                    painter.scale(-1.0, 1.0)

                layer_painter.paint(painter)
                painter.end()
            except Exception as e:
                if self.isFirstPaint:
                    logger.error(__name__ + ": Exception when painting " +
                                 layer_painter.name + " layer: ")
                    logger.error(__name__ + ": " + str(e))
                    import traceback
                    print(traceback.format_exc())
            if self.isFirstPaint:
                logger.debug(__name__ + ": First painting of " +
                             layer_painter.name + " layer took: " +
                             logger.timerLogStr(layerPaintTime))
        if self.isFirstPaint:
            logger.debug(__name__ + ": First painting of map took: " +
                         logger.timerLogStr(mapPaintTime))
            self.isFirstPaint = False
