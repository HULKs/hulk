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
    name = "striker"

    def __init__(self, layer_model: ty.Dict, nao: Nao):
        merged_model = ui_utils.load_model(os.path.dirname(__file__) +
                                           "/model.json", layer_model)
        super(Main, self).__init__(merged_model, nao, str(uuid.uuid4()))

        self.kickRatingChunks = []
        self.kickRatingChunks_identifier = uuid.uuid4()
        self.hitPoints = None
        self.hitPoints_identifier = uuid.uuid4()
        self.teamBallPosition = [0, 0]
        self.teamBallPosition_identifier = uuid.uuid4()
        self.rateKick = False
        self.rateKick_identifier = uuid.uuid4()
        self.kickRatingChunkWeights = []
        self.kickRatingChunkWeights_identifier = uuid.uuid4()
        self.firstShadowPoint = None
        self.firstShadowPoint_identifier = uuid.uuid4()
        self.secondShadowPoint = None
        self.secondShadowPoint_identifier = uuid.uuid4()
        self.firstShadowPointAfter = None
        self.firstShadowPointAfter_identifier = uuid.uuid4()
        self.secondShadowPointAfter = None
        self.secondShadowPointAfter_identifier = uuid.uuid4()

        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: Nao):
        self.nao = nao
        self.subscribe()

    def update_kickRatingChunks(self, data):
        self.kickRatingChunks = data.data

    def update_hitPoints(self, data):
        self.hitPoints = data.data

    def update_teamBallPosition(self, data):
        self.teamBallPosition = data.data["position"]

    def update_rateKick(self, data):
        self.rateKick = data.data

    def update_kickRatingChunkWeights(self, data):
        self.kickRatingChunkWeights = data.data

    def update_firstShadowPoint(self, data):
        self.firstShadowPoint = data.data

    def update_secondShadowPoint(self, data):
        self.secondShadowPoint = data.data

    def update_firstShadowPointAfter(self, data):
        self.firstShadowPointAfter = data.data

    def update_secondShadowPointAfter(self, data):
        self.secondShadowPointAfter = data.data

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["kickRatingChunksKey"],
            self.kickRatingChunks_identifier,
            lambda i: self.update_kickRatingChunks(i))
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["kickRatingChunkWeightsKey"],
            self.kickRatingChunkWeights_identifier,
            lambda i: self.update_kickRatingChunkWeights(i))
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["rateKickKey"],
            self.rateKick_identifier,
            lambda i: self.update_rateKick(i))
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["hitPointsKey"],
            self.hitPoints_identifier,
            lambda i: self.update_hitPoints(i))
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["teamBallPositionKey"],
            self.teamBallPosition_identifier,
            lambda i: self.update_teamBallPosition(i))
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["firstShadowPointKey"],
            self.firstShadowPoint_identifier,
            lambda i: self.update_firstShadowPoint(i))
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["secondShadowPointKey"],
            self.secondShadowPoint_identifier,
            lambda i: self.update_secondShadowPoint(i))
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["firstShadowPointAfterKey"],
            self.firstShadowPointAfter_identifier,
            lambda i: self.update_firstShadowPointAfter(i))
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["secondShadowPointAfterKey"],
            self.secondShadowPointAfter_identifier,
            lambda i: self.update_secondShadowPointAfter(i))

    def paint(self, painter: Painter):
        painter.setPen(qtg.QPen(qtc.Qt.black, 0))
        painter.drawPose([self.teamBallPosition, 0.0],
                         0, 0, str(self.rateKick), 0.15, [0.1, 0.1])

        painter.setBrush(qtg.QColor("#000000"))
        if self.firstShadowPoint is not None and self.secondShadowPoint is not None:
            painter.drawPose([self.firstShadowPoint, 0.0],
                             0.1, 0, "first", 0.1)
            painter.drawPose([self.secondShadowPoint, 0.0],
                             0.1, 0, "second", 0.1)
        if self.firstShadowPointAfter is not None and self.secondShadowPointAfter is not None:
            painter.drawPose([self.firstShadowPointAfter, 0.0],
                             0.1, 0, "firstAfter", 0.1)
            painter.drawPose([self.secondShadowPointAfter, 0.0],
                             0.1, 0, "secondAfter", 0.1)

        for index, chunk in enumerate(self.kickRatingChunks):
            if chunk:
                color = qtg.QColor("#00ff00")
            else:
                color = qtg.QColor("#ff0000")
            painter.setBrush(color)
            if self.hitPoints is not None:
                painter.drawPose(
                    [self.hitPoints[index], 0.0],
                    0.1,
                    0,
                    "{0:.2f}".format(self.kickRatingChunkWeights[index]),
                    0.1,
                    [0.1, 0])

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["kickRatingChunksKey"],
                self.kickRatingChunks_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["kickRatingChunkWeightsKey"],
                self.kickRatingChunkWeights_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["rateKickKey"],
                self.rateKick_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["hitPointsKey"],
                self.hitPoints_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["teamBallPositionKey"],
                self.teamBallPosition_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["firstShadowPointKey"],
                self.firstShadowPoint_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["secondShadowPointKey"],
                self.secondShadowPoint_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["firstShadowPointAfterKey"],
                self.firstShadowPointAfter_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["secondShadowPointAfterKey"],
                self.secondShadowPointAfter_identifier)
