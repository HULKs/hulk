import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import uuid
import math
import enum

from mate.ui.views.map.map_painter import Painter
from mate.ui.views.map.layer.layer import Layer
import mate.net.nao as nao


class ObstacleTypes(enum.Enum):
    goalPost = 0
    unknown = 1
    anonymousRobot = 2
    hostileRobot = 3
    teamRobot = 4
    fallenAnonymousRobot = 5
    fallenHostileRobot = 6
    fallenTeamRobot = 7
    ball = 8
    freeKickArea = 9


class ObstacleData(Layer):
    def __init__(self, layer: ty.Dict, nao: nao.Nao):
        self.nao = nao
        self.layer = layer
        self.identifier = uuid.uuid4()
        self.obstacles = None
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

    def update_obstacles(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["obstacles"]["key_lambda"], scope)
        self.obstacles = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["transformation"]["key"],
            self.identifier,
            lambda i: self.update_transformation(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["obstacles"]["key"],
            self.identifier,
            lambda i: self.update_obstacles(i)
        )

    def penForObstacleType(self, obstacleType):
        color = self.layer["settings"
                           ]["obstacles"
                             ][ObstacleTypes(obstacleType).name + "Color"]
        width = self.layer["settings"]["obstacles"]["penWidth"]
        return qtg.QPen(qtg.QColor(color), width, qtc.Qt.DashLine)

    def paint(self, painter: Painter):
        painter.transformByPose(self.transformation)
        if self.obstacles is not None:
            # Obstacles
            painter.setBrush(qtc.Qt.NoBrush)
            for obstacle in self.obstacles:
                pen = self.penForObstacleType(obstacle["type"])
                painter.setPen(pen)
                painter.drawEllipse(
                    qtc.QPointF(obstacle["relativePosition"][0],
                                obstacle["relativePosition"][1]),
                    obstacle["radius"],
                    obstacle["radius"]
                )

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["transformation"]["key"],
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["obstacles"]["key"],
                self.identifier)
