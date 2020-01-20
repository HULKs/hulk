import typing as ty
import uuid
import os

import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import math

from mate.ui.panels.map.map_painter import Painter
from mate.ui.panels.map.layer._layer_main import _Layer
from mate.net.nao import Nao
import mate.ui.utils as ui_utils


class Main(_Layer):
    name = "coordinateSystem"

    def __init__(self, layer_model: ty.Dict, nao: Nao):
        merged_model = ui_utils.load_model(os.path.dirname(__file__) +
                                           "/model.json", layer_model)
        super(Main, self).__init__(merged_model, nao, str(uuid.uuid4()))

    def connect(self, nao: Nao):
        pass

    def destroy(self):
        pass

    def paint(self, painter: Painter):
        # Draw background
        bgColor = qtg.QColor(self.config["coordinateSystem"
                                         ]["backgroundColor"])
        bgColor.setAlpha(self.config["coordinateSystem"]["backgroundAlpha"])
        painter.setBrush(bgColor)
        painter.setPen(qtc.Qt.NoPen)
        painter.drawRectF(
            -self.config["coordinateSystem"]["width"]/2,
            -self.config["coordinateSystem"]["height"]/2,
            self.config["coordinateSystem"]["width"],
            self.config["coordinateSystem"]["height"]
        )
        # Draw Axis
        lineColor = qtg.QColor(self.config["coordinateSystem"]["lineColor"])
        pen = qtg.QPen(lineColor)
        pen.setWidthF(self.config["coordinateSystem"]["lineWidth"])
        painter.setPen(pen)
        painter.setBrush(qtc.Qt.NoBrush)
        painter.drawLineF(0,
                          -self.config["coordinateSystem"]["height"]/2,
                          0,
                          self.config["coordinateSystem"]["height"]/2)
        painter.drawLineF(-self.config["coordinateSystem"]["width"]/2,
                          0,
                          self.config["coordinateSystem"]["width"]/2,
                          0)
        if self.config["coordinateSystem"]["polar"]:
            # Draw Circles
            r = 0.0
            while (r + self.config["coordinateSystem"]["radialStepSize"] <=
                   max(self.config["coordinateSystem"]["height"]/2,
                       self.config["coordinateSystem"]["width"]/2)):
                r += self.config["coordinateSystem"]["radialStepSize"]
                painter.drawEllipse(qtc.QPointF(0, 0), r, r)
            # Draw Angles
            l = max(self.config["coordinateSystem"]["height"]/2,
                    self.config["coordinateSystem"]["width"]/2)
            deg = 90.0
            while deg + self.config["coordinateSystem"
                                    ]["polarAngleStepSize"] < 270.0:
                deg += self.config["coordinateSystem"]["polarAngleStepSize"]
                v = [math.cos(math.radians(deg))*l,
                     math.sin(math.radians(deg))*l]
                painter.drawLineF(0, 0, v[0], v[1])
                painter.drawLineF(0, 0, -v[0], v[1])
        else:
            # draw grid
            # horizontal lines
            offset = 0.0
            while (offset + self.config["coordinateSystem"]["stepSizeY"] <=
                   self.config["coordinateSystem"]["height"]/2):
                offset += self.config["coordinateSystem"]["stepSizeY"]
                painter.drawLineF(-self.config["coordinateSystem"]["width"]/2,
                                  offset,
                                  self.config["coordinateSystem"]["width"]/2,
                                  offset)
                painter.drawLineF(-self.config["coordinateSystem"]["width"]/2,
                                  -offset,
                                  self.config["coordinateSystem"]["width"]/2,
                                  -offset)
            # vertical lines
            offset = 0.0
            while (offset + self.config["coordinateSystem"]["stepSizeX"] <=
                   self.config["coordinateSystem"]["width"]/2):
                offset += self.config["coordinateSystem"]["stepSizeX"]
                painter.drawLineF(offset,
                                  -self.config["coordinateSystem"]["height"]/2,
                                  offset,
                                  self.config["coordinateSystem"]["height"]/2)
                painter.drawLineF(-offset,
                                  -self.config["coordinateSystem"]["height"]/2,
                                  -offset,
                                  self.config["coordinateSystem"]["height"]/2)
