import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import uuid
import math

from mate.ui.views.map.map_painter import Painter
from mate.ui.views.map.layer.layer import Layer
import mate.net.nao as nao


class LineData(Layer):
    def __init__(self, layer: ty.Dict, nao: nao.Nao):
        self.nao = nao
        self.layer = layer
        self.identifier = uuid.uuid4()
        self.lines = None
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

    def update_lines(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["lines"]["key_lambda"], scope)
        self.lines = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["transformation"]["key"],
            self.identifier,
            lambda i: self.update_transformation(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["lines"]["key"],
            self.identifier,
            lambda i: self.update_lines(i)
        )

    def paint(self, painter: Painter):
        painter.transformByPose(self.transformation)
        painter.setPen(qtg.QColor(
                           self.layer["settings"]["lines"]["lineColor"]),
                       self.layer["settings"]["lines"]["lineWidth"])
        if self.lines is not None:
            for line in self.lines:
                painter.drawLineF(line[0][0],
                                  line[0][1],
                                  line[1][0],
                                  line[1][1])

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["transformation"]["key"],
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["lines"]["key"],
                self.identifier)
