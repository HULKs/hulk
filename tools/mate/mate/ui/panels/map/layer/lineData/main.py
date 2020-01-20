import typing as ty
import PyQt5.QtGui as qtg

import uuid
import os

from mate.ui.panels.map.map_painter import Painter
from mate.ui.panels.map.layer._layer_main import _Layer
from mate.net.nao import Nao
import mate.ui.utils as ui_utils


class Main(_Layer):
    name = "lineData"

    def __init__(self, layer_model: ty.Dict, nao: Nao):
        merged_model = ui_utils.load_model(os.path.dirname(__file__) +
                                           "/model.json", layer_model)
        super(Main, self).__init__(merged_model, nao, str(uuid.uuid4()))

        self.lines = None
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

    def update_lines(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]["lines"]["key_lambda"], scope)
        self.lines = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["transformation"]["key"],
            self.transformation_identifier,
            lambda i: self.update_transformation(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["lines"]["key"],
            self.identifier,
            lambda i: self.update_lines(i)
        )

    def paint(self, painter: Painter):
        painter.transformByPose(self.transformation)
        painter.setPen(qtg.QColor(
                           self.layer_model["config"]["lines"]["lineColor"]),
                       self.layer_model["config"]["lines"]["lineWidth"])
        if self.lines is not None:
            for line in self.lines:
                painter.drawLineF(line[0][0],
                                  line[0][1],
                                  line[1][0],
                                  line[1][1])

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["transformation"]["key"],
                self.transformation_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["lines"]["key"],
                self.identifier)
