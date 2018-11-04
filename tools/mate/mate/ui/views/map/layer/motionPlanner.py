import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import uuid
import math

from mate.ui.views.map.map_painter import Painter
from mate.ui.views.map.layer.layer import Layer
import mate.net.nao as nao


class MotionPlanner(Layer):
    def __init__(self, layer: ty.Dict, nao: nao.Nao):
        self.nao = nao
        self.layer = layer
        self.identifier = uuid.uuid4()
        self.motionPlan = None
        self.transformation = [[0, 0], 0]
        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: nao.Nao):
        self.nao = nao
        self.subscribe()

    def update_transformation(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["transformation"]["key_lambda"], scope)
        self.transformation = scope["output"]

    def update_motionPlan(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["motionPlan"]["key_lambda"], scope)
        self.motionPlan = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["transformation"]["key"],
            self.identifier,
            lambda i: self.update_transformation(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["motionPlan"]["key"],
            self.identifier,
            lambda i: self.update_motionPlan(i)
        )

    def paint(self, painter: Painter):
        painter.transformByPose(self.transformation)
        if self.motionPlan is not None:
            # Walking-target
            painter.setBrush(qtc.Qt.NoBrush)
            painter.setPen(qtg.QColor(self.layer["settings"
                                                 ]["motionPlan"
                                                   ]["targetColor"]),
                           0)
            painter.drawTarget(self.motionPlan["walkTarget"],
                               self.layer["settings"
                                          ]["motionPlan"
                                            ]["targetCircleDiameter"])
            # Dotted line to walking-target
            dotted_pen = qtg.QPen(qtc.Qt.yellow,
                                  0,
                                  qtc.Qt.DashDotLine)
            painter.setPen(dotted_pen)
            painter.drawLineF(0.0,
                              0.0,
                              self.motionPlan["walkTarget"][0][0],
                              self.motionPlan["walkTarget"][0][1])
            # Translation
            painter.setPen(qtg.QColor("#ff0000"), 0)
            painter.drawLineF(0.0,
                              0.0,
                              self.motionPlan["translation"][0],
                              self.motionPlan["translation"][1])

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["transformation"]["key"],
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["motionPlan"]["key"],
                self.identifier)
