import typing as ty
import uuid
import os
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

from mate.ui.panels.map.map_painter import Painter
from mate.ui.panels.map.layer._layer_main import _Layer
import mate.ui.utils as ui_utils
from mate.net.nao import Nao


class Main(_Layer):
    name = "field"

    def __init__(self, layer_model: ty.Dict, nao: Nao):
        merged_model = ui_utils.load_model(os.path.dirname(__file__) +
                                           "/model.json", layer_model)
        super(Main, self).__init__(merged_model, nao, str(uuid.uuid4()))

        self.fieldScale = [
            (self.config["field"]["length"] + self.config["field"]["borderStripWidth"] * 2.0) /
            self.config["field"]["length"],
            (self.config["field"]["width"] + self.config["field"]["borderStripWidth"] * 2.0) /
            self.config["field"]["width"],
        ]

    def connect(self, nao: Nao):
        pass

    def destroy(self):
        pass

    def paint(self, painter: Painter):
        pen = qtg.QPen(qtc.Qt.white)
        pen.setWidthF(self.config["field"]["lineWidth"])

        # This creates sharp edges where lines join instead of rounded corners
        pen.setJoinStyle(qtc.Qt.MiterJoin)

        # green background
        if not self.config["field"]["hide_background"]:
            painter.setBrush(qtc.Qt.darkGreen)
            painter.setPen(qtc.Qt.NoPen)
            painter.drawRectF(
                -self.config["field"]["length"] / 2 -
                self.config["field"]["borderStripWidth"],
                -self.config["field"]["width"] / 2 -
                self.config["field"]["borderStripWidth"],
                self.config["field"]["length"] +
                self.config["field"]["borderStripWidth"] * 2,
                self.config["field"]["width"] +
                self.config["field"]["borderStripWidth"] * 2
            )

        # white pen
        painter.setPen(pen)
        painter.setBrush(qtc.Qt.NoBrush)

        # field border
        painter.drawRectF(
            -self.config["field"]["length"] / 2,
            -self.config["field"]["width"] / 2,
            self.config["field"]["length"],
            self.config["field"]["width"])

        # goal box area left
        painter.drawRectF(
            -self.config["field"]["length"] / 2,
            -self.config["field"]["goalBoxAreaWidth"] / 2,
            self.config["field"]["goalBoxAreaLength"],
            self.config["field"]["goalBoxAreaWidth"])

        # goal box area right
        painter.drawRectF(
            self.config["field"]["length"] / 2,
            -self.config["field"]["goalBoxAreaWidth"] / 2,
            -self.config["field"]["goalBoxAreaLength"],
            self.config["field"]["goalBoxAreaWidth"])

        # penalty area left
        painter.drawRectF(
            -self.config["field"]["length"] / 2,
            -self.config["field"]["penaltyAreaWidth"] / 2,
            self.config["field"]["penaltyAreaLength"],
            self.config["field"]["penaltyAreaWidth"])

        # penalty area right
        painter.drawRectF(
            self.config["field"]["length"] / 2,
            -self.config["field"]["penaltyAreaWidth"] / 2,
            -self.config["field"]["penaltyAreaLength"],
            self.config["field"]["penaltyAreaWidth"])

        # center line
        painter.drawLineF(
            0,
            -self.config["field"]["width"] / 2,
            0,
            self.config["field"]["width"] / 2)

        # center circle
        painter.drawEllipse(
            qtc.QPointF(0, 0),
            self.config["field"]["centerCircleDiameter"] / 2,
            self.config["field"]["centerCircleDiameter"] / 2)

        # penalty mark left
        painter.setBrush(qtc.Qt.white)
        painter.drawEllipse(
            qtc.QPointF(-self.config["field"]["length"] / 2 +
                        self.config["field"]["penaltyMarkerDistance"],
                        0),
            self.config["field"]["penaltyMarkerSize"] / 2,
            self.config["field"]["penaltyMarkerSize"] / 2)

        # penalty mark right
        painter.drawEllipse(
            qtc.QPointF(self.config["field"]["length"] / 2 -
                        self.config["field"]["penaltyMarkerDistance"],
                        0),
            self.config["field"]["penaltyMarkerSize"] / 2,
            self.config["field"]["penaltyMarkerSize"] / 2)

        # kick off pointEllipse
        painter.drawEllipse(
            qtc.QPointF(0, 0),
            self.config["field"]["penaltyMarkerSize"] / 2,
            self.config["field"]["penaltyMarkerSize"] / 2)
