import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc
import uuid
import enum
import math

from mate.ui.views.map.map_painter import Painter
from mate.ui.views.map.layer.layer import Layer
import mate.net.nao as nao


class Striker(Layer):
    def __init__(self, layer: ty.Dict, nao: nao.Nao):
        self.nao = nao
        self.layer = layer
        self.identifier = uuid.uuid4()
        self.kickRatingChunks = []
        self.hitPoints = None
        self.teamBallPosition = [0, 0]
        self.rateKick = False
        self.kickRatingChunkWeights = []
        self.firstShadowPoint = None
        self.secondShadowPoint = None
        self.firstShadowPointAfter = None
        self.secondShadowPointAfter = None

        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: nao.Nao):
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
            self.layer["settings"]["kickRatingChunksKey"], self.identifier,
            lambda i: self.update_kickRatingChunks(i))
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["kickRatingChunkWeightsKey"], self.identifier,
            lambda i: self.update_kickRatingChunkWeights(i))
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["rateKickKey"], self.identifier,
            lambda i: self.update_rateKick(i))
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["hitPointsKey"], self.identifier,
            lambda i: self.update_hitPoints(i))
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["teamBallPositionKey"], self.identifier,
            lambda i: self.update_teamBallPosition(i))
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["firstShadowPointKey"], self.identifier,
            lambda i: self.update_firstShadowPoint(i))
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["secondShadowPointKey"], self.identifier,
            lambda i: self.update_secondShadowPoint(i))
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["firstShadowPointAfterKey"], self.identifier,
            lambda i: self.update_firstShadowPointAfter(i))
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["secondShadowPointAfterKey"], self.identifier,
            lambda i: self.update_secondShadowPointAfter(i))

    def paint(self, painter: Painter):
        painter.setPen(qtg.QPen(qtc.Qt.black, 0))
        painter.drawPose([self.teamBallPosition, 0.0], 0, 0, str(self.rateKick), 0.15, [0.1, 0.1])

        painter.setBrush(qtg.QColor("#000000"))
        if self.firstShadowPoint is not None and self.secondShadowPoint is not None:
            painter.drawPose([self.firstShadowPoint, 0.0], 0.1, 0, "first", 0.1)
            painter.drawPose([self.secondShadowPoint, 0.0], 0.1, 0, "second", 0.1)
        if self.firstShadowPointAfter is not None and self.secondShadowPointAfter is not None:
            painter.drawPose([self.firstShadowPointAfter, 0.0], 0.1, 0, "firstAfter", 0.1)
            painter.drawPose([self.secondShadowPointAfter, 0.0], 0.1, 0, "secondAfter", 0.1)

        for index, chunk in enumerate(self.kickRatingChunks):
            if chunk:
                color = qtg.QColor("#00ff00")
            else:
                color = qtg.QColor("#ff0000")
            painter.setBrush(color)
            if self.hitPoints is not None:
                painter.drawPose([self.hitPoints[index], 0.0], 0.1, 0, "{0:.2f}".format(self.kickRatingChunkWeights[index]), 0.1, [0.1, 0])

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["kickRatingChunksKey"], self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["kickRatingChunkWeightsKey"], self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["rateKickKey"], self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["hitPointsKey"], self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["teamBallPositionKey"], self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["firstShadowPointKey"], self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["secondShadowPointKey"], self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["firstShadowPointAfterKey"], self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["secondShadowPointAfterKey"], self.identifier)

