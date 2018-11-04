import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

from mate.ui.views.map.map_painter import Painter
from mate.ui.views.map.layer.layer import Layer
import mate.net.nao as nao


class Field(Layer):
    def __init__(self, layer: ty.Dict, nao: nao.Nao):
        self.layer = layer
        self.field_settings = self.layer["settings"]["field"]

    def connect(self, nao):
        pass

    def destroy(self):
        pass

    def paint(self, painter: Painter):
        pen = qtg.QPen(qtc.Qt.white)
        pen.setWidthF(self.field_settings["lineWidth"])

        # This creates sharp edges where lines join instead of rounded corners
        pen.setJoinStyle(qtc.Qt.MiterJoin)

        # green background
        painter.setBrush(qtc.Qt.darkGreen)
        painter.setPen(qtc.Qt.NoPen)
        painter.drawRectF(
            -self.field_settings["length"] / 2 -
            self.field_settings["borderStripWidth"],
            -self.field_settings["width"] / 2 -
            self.field_settings["borderStripWidth"],
            self.field_settings["length"] +
            self.field_settings["borderStripWidth"] * 2,
            self.field_settings["width"] +
            self.field_settings["borderStripWidth"] * 2
        )

        # white pen
        painter.setPen(pen)
        painter.setBrush(qtc.Qt.NoBrush)

        # field border
        painter.drawRectF(
            -self.field_settings["length"] / 2,
            -self.field_settings["width"] / 2,
            self.field_settings["length"],
            self.field_settings["width"])

        # penalty area left
        painter.drawRectF(
            -self.field_settings["length"] / 2,
            -self.field_settings["penaltyAreaWidth"] / 2,
            self.field_settings["penaltyAreaLength"],
            self.field_settings["penaltyAreaWidth"])

        # penalty area right
        painter.drawRectF(
            self.field_settings["length"] / 2,
            -self.field_settings["penaltyAreaWidth"] / 2,
            -self.field_settings["penaltyAreaLength"],
            self.field_settings["penaltyAreaWidth"])

        # center line
        painter.drawLineF(
            0,
            -self.field_settings["width"] / 2,
            0,
            self.field_settings["width"] / 2)

        # center circle
        painter.drawEllipse(
            qtc.QPointF(0, 0),
            self.field_settings["centerCircleDiameter"] / 2,
            self.field_settings["centerCircleDiameter"] / 2)

        # penalty mark left
        painter.setBrush(qtc.Qt.white)
        painter.drawEllipse(
            qtc.QPointF(-self.field_settings["length"] / 2 +
                        self.field_settings["penaltyMarkerDistance"],
                        0),
            self.field_settings["penaltyMarkerSize"] / 2,
            self.field_settings["penaltyMarkerSize"] / 2)

        # penalty mark right
        painter.drawEllipse(
            qtc.QPointF(self.field_settings["length"] / 2 -
                        self.field_settings["penaltyMarkerDistance"],
                        0),
            self.field_settings["penaltyMarkerSize"] / 2,
            self.field_settings["penaltyMarkerSize"] / 2)

        # kick off pointEllipse
        painter.drawEllipse(
            qtc.QPointF(0, 0),
            self.field_settings["penaltyMarkerSize"] / 2,
            self.field_settings["penaltyMarkerSize"] / 2)
