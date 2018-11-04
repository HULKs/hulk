import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import math

from mate.ui.views.map.map_painter import Painter
from mate.ui.views.map.layer.layer import Layer
import mate.net.nao as nao


class CoordinateSystem(Layer):
    def __init__(self, layer: ty.Dict, nao: nao.Nao):
        self.layer = layer
        self.settings = self.layer["settings"]["coordinateSystem"]

    def connect(self, nao):
        pass

    def destroy(self):
        pass

    def paint(self, painter: Painter):
        # Draw background
        bgColor = qtg.QColor(self.settings["backgroundColor"])
        bgColor.setAlpha(self.settings["backgroundAlpha"])
        painter.setBrush(bgColor)
        painter.setPen(qtc.Qt.NoPen)
        painter.drawRectF(
            -self.settings["width"]/2,
            -self.settings["height"]/2,
            self.settings["width"],
            self.settings["height"]
        )
        # Draw Axis
        lineColor = qtg.QColor(self.settings["lineColor"])
        pen = qtg.QPen(lineColor)
        pen.setWidthF(self.settings["lineWidth"])
        painter.setPen(pen)
        painter.setBrush(qtc.Qt.NoBrush)
        painter.drawLineF(0,
                          -self.settings["height"]/2,
                          0,
                          self.settings["height"]/2)
        painter.drawLineF(-self.settings["width"]/2,
                          0,
                          self.settings["width"]/2,
                          0)
        if self.settings["polar"]:
            # Draw Circles
            r = 0.0
            while (r + self.settings["radialStepSize"] <=
                   max(self.settings["height"]/2, self.settings["width"]/2)):
                r += self.settings["radialStepSize"]
                painter.drawEllipse(qtc.QPointF(0, 0), r, r)
            # Draw Angles
            l = max(self.settings["height"]/2, self.settings["width"]/2)
            deg = 90.0
            while (deg + self.settings["polarAngleStepSize"] < 270.0):
                deg += self.settings["polarAngleStepSize"]
                v = [math.cos(math.radians(deg))*l,
                     math.sin(math.radians(deg))*l]
                painter.drawLineF(0, 0, v[0], v[1])
                painter.drawLineF(0, 0, -v[0], v[1])
        else:
            # draw grid
            # horizontal lines
            offset = 0.0
            while (offset + self.settings["stepSizeY"] <=
                   self.settings["height"]/2):
                offset += self.settings["stepSizeY"]
                painter.drawLineF(-self.settings["width"]/2,
                                  offset,
                                  self.settings["width"]/2,
                                  offset)
                painter.drawLineF(-self.settings["width"]/2,
                                  -offset,
                                  self.settings["width"]/2,
                                  -offset)
            # vertical lines
            offset = 0.0
            while (offset + self.settings["stepSizeX"] <=
                   self.settings["width"]/2):
                offset += self.settings["stepSizeX"]
                painter.drawLineF(offset,
                                  -self.settings["height"]/2,
                                  offset,
                                  self.settings["height"]/2)
                painter.drawLineF(-offset,
                                  -self.settings["height"]/2,
                                  -offset,
                                  self.settings["height"]/2)
