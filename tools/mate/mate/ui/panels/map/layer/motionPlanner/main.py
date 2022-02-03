import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import uuid
import os

from mate.ui.panels.map.map_painter import Painter
from mate.ui.panels.map.layer._layer_main import _Layer
from mate.net.nao import Nao
import mate.ui.utils as ui_utils


class Main(_Layer):
    name = "motionPlanner"

    def __init__(self, layer_model: ty.Dict, nao: Nao):
        merged_model = ui_utils.load_model(os.path.dirname(__file__) +
                                           "/model.json", layer_model)
        super(Main, self).__init__(merged_model, nao, str(uuid.uuid4()))

        self.targetPosition = None
        self.displacementVector = None
        self.transformation = [[0, 0], 0]
        self.transformation_identifier = uuid.uuid4()
        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: Nao):
        self.nao = nao
        self.subscribe()

    def update_transformation(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]["transformation"]["key_lambda"], scope)
        self.transformation = scope["output"]

    def update_targetPosition(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]["targetPosition"]["key_lambda"], scope)
        self.targetPosition = scope["output"]

    def update_displacementVector(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]
             ["displacementVector"]["key_lambda"], scope)
        self.displacementVector = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["transformation"]["key"],
            self.transformation_identifier,
            lambda i: self.update_transformation(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["targetPosition"]["key"],
            self.identifier,
            lambda i: self.update_targetPosition(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["displacementVector"]["key"],
            self.identifier,
            lambda i: self.update_displacementVector(i)
        )

    def paint(self, painter: Painter):
        painter.transformByPose(self.transformation)
        painter.setBrush(qtc.Qt.NoBrush)
        if self.targetPosition is not None:
            painter.setPen(qtg.QColor(
                self.layer_model["config"]["targetPosition"]["targetColor"]), 0)
            painter.drawTarget([self.targetPosition, 0], self.layer_model["config"]
                               ["targetPosition"]["targetCircleDiameter"])
        if self.displacementVector is not None and self.displacementVector[0] != 0 and self.displacementVector[1] != 0:
            painter.setPen(qtg.QPen(qtg.QColor(self.layer_model["config"]["displacementVector"]["lineColor"]),
                                    self.layer_model["config"]["displacementVector"]["lineWidth"], qtc.Qt.DashDotLine))
            painter.drawLineF(0.0,
                              0.0,
                              self.displacementVector[0],
                              self.displacementVector[1])
        if self.targetPosition is not None and self.displacementVector is not None:
            painter.setPen(qtc.Qt.yellow, 0)
            painter.drawTarget([[self.targetPosition[0] - self.displacementVector[0],
                                 self.targetPosition[1] - self.displacementVector[1]], 0], self.layer_model["config"]["targetPosition"]["targetCircleDiameter"])

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["transformation"]["key"],
                self.transformation_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["targetPosition"]["key"],
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["displacementVector"]["key"],
                self.identifier)
