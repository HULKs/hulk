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
    name = "ballPosition"

    def __init__(self, layer_model: ty.Dict, nao: Nao):
        merged_model = ui_utils.load_model(os.path.dirname(__file__) +
                                           "/model.json", layer_model)
        super(Main, self).__init__(merged_model, nao, str(uuid.uuid4()))

        self.position = None
        self.transformation = [[0, 0], 0]
        self.transformation_identifier = uuid.uuid4()
        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: Nao):
        self.nao = nao
        self.subscribe()

    def update_transformation(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.config["transformation"]["key_lambda"], scope)
        self.transformation = scope["output"]

    def update_position(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.config["position"]["key_lambda"], scope)
        self.position = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.config["transformation"]["key"],
            self.transformation_identifier,
            lambda i: self.update_transformation(i)
        )
        self.nao.debug_protocol.subscribe(
            self.config["position"]["key"],
            self.identifier,
            lambda i: self.update_position(i)
        )

    def paint(self, painter: Painter):
        painter.transformByPose(self.transformation)
        painter.setPen(qtc.Qt.black, 0)
        painter.setBrush(qtg.QColor(
            self.config["position"]["color"]))
        if self.position is not None:
            painter.drawEllipse(
                qtc.QPointF(self.position[0], self.position[1]),
                self.config["position"]["circleDiameter"] / 2,
                self.config["position"]["circleDiameter"] / 2)

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.config["transformation"]["key"],
                self.transformation_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.config["position"]["key"],
                self.identifier)
