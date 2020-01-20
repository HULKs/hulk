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

        self.motionPlan = None
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

    def update_motionPlan(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]["motionPlan"]["key_lambda"], scope)
        self.motionPlan = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["transformation"]["key"],
            self.transformation_identifier,
            lambda i: self.update_transformation(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["motionPlan"]["key"],
            self.identifier,
            lambda i: self.update_motionPlan(i)
        )

    def paint(self, painter: Painter):
        painter.transformByPose(self.transformation)
        if self.motionPlan is not None:
            # Walking-target
            painter.setBrush(qtc.Qt.NoBrush)
            painter.setPen(qtg.QColor(self.layer_model["config"
                                                       ]["motionPlan"
                                                         ]["targetColor"]),
                           0)
            painter.drawTarget(self.motionPlan["walkTarget"],
                               self.layer_model["config"
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
                self.layer_model["config"]["transformation"]["key"],
                self.transformation_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["motionPlan"]["key"],
                self.identifier)
