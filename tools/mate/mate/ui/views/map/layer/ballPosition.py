import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import uuid
import math

from mate.ui.views.map.map_painter import Painter
from mate.ui.views.map.layer.layer import Layer
import mate.net.nao as nao


class BallPosition(Layer):
    def __init__(self, layer: ty.Dict, nao: nao.Nao):
        self.nao = nao
        self.layer = layer
        self.identifier = uuid.uuid4()
        self.position = None
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

    def update_position(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["position"]["key_lambda"], scope)
        self.position = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["transformation"]["key"],
            self.identifier,
            lambda i: self.update_transformation(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["position"]["key"],
            self.identifier,
            lambda i: self.update_position(i)
        )

    def paint(self, painter: Painter):
        painter.transformByPose(self.transformation)
        painter.setPen(qtc.Qt.black, 0)
        painter.setBrush(qtg.QColor(
            self.layer["settings"]["position"]["color"]))
        if self.position is not None:
            painter.drawEllipse(
                qtc.QPointF(self.position[0], self.position[1]),
                self.layer["settings"]["position"]
                          ["circleDiameter"] / 2,
                self.layer["settings"]["position"]
                          ["circleDiameter"] / 2
            )

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["transformation"]["key"],
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["position"]["key"],
                self.identifier)
