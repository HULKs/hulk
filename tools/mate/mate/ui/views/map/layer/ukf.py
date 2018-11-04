import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import uuid

from mate.ui.views.map.map_painter import Painter
from mate.ui.views.map.layer.layer import Layer
import mate.net.nao as nao


class UKF(Layer):
    def __init__(self, layer: ty.Dict, nao: nao.Nao):
        self.nao = nao
        self.layer = layer
        self.identifier = uuid.uuid4()

        self.poseHypotheses = None
        self.publishedPose = None

        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: nao.Nao):
        self.nao = nao
        self.subscribe()

    def update_pose2DEstimator(self, data):
        self.poseHypotheses = data.data["poseHypotheses"]
        self.publishedPose = data.data["publishedPose"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["ukf"]["key"], self.identifier,
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
                self.layer["settings"]["ukf"]["key"], self.identifier)
