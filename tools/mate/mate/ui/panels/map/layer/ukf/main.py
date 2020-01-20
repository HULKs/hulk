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
    name = "ukf"

    def __init__(self, layer_model: ty.Dict, nao: Nao):
        merged_model = ui_utils.load_model(os.path.dirname(__file__) +
                                           "/model.json", layer_model)
        super(Main, self).__init__(merged_model, nao, str(uuid.uuid4()))

        self.poseHypotheses = None
        self.publishedPose = None

        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: Nao):
        self.nao = nao
        self.subscribe()

    def update_pose2DEstimator(self, data):
        self.poseHypotheses = data.data["poseHypotheses"]
        self.publishedPose = data.data["publishedPose"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["ukf"]["key"], self.identifier,
            lambda i: self.update_pose2DEstimator(i))

    def paint(self, painter: Painter):
        if not (self.poseHypotheses and self.publishedPose):
            return

        painter.setPen(qtg.QPen(qtc.Qt.black, 0))
        painter.setBrush(qtc.Qt.blue)

        painter.drawPose(self.publishedPose, 0.2, 0.2)

        for hypothesis in self.poseHypotheses:
            self.drawHypothesis(painter, hypothesis)

    def drawHypothesis(self, painter: Painter, hypothesis):
        painter.setBrush(qtc.Qt.yellow)
        for sigmaPoint in hypothesis["sigmaPoints"]:
            painter.drawPose(painter.getPoseFromVector3(sigmaPoint),
                             0.15,
                             0.15)
        painter.setBrush(qtc.Qt.red)
        painter.drawPose(
            painter.getPoseFromVector3(hypothesis["stateMean"]),
            0.2,
            0.2,
            "{:.2f}".format(hypothesis["meanEvalError"]))

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["ukf"]["key"], self.identifier)
