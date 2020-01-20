import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import uuid
import enum
import os

from mate.ui.panels.map.map_painter import Painter
from mate.ui.panels.map.layer._layer_main import _Layer
from mate.net.nao import Nao
import mate.ui.utils as ui_utils


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


class Main(_Layer):
    name = "obstacleData"

    def __init__(self, layer_model: ty.Dict, nao: Nao):
        merged_model = ui_utils.load_model(os.path.dirname(__file__) +
                                           "/model.json", layer_model)
        super(Main, self).__init__(merged_model, nao, str(uuid.uuid4()))

        self.obstacles = None
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

    def update_obstacles(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]["obstacles"]["key_lambda"], scope)
        self.obstacles = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["transformation"]["key"],
            self.transformation_identifier,
            lambda i: self.update_transformation(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["obstacles"]["key"],
            self.identifier,
            lambda i: self.update_obstacles(i)
        )

    def penForObstacleType(self, obstacleType):
        color = self.layer_model["config"
                                 ]["obstacles"
                                   ][ObstacleTypes(obstacleType).name +
                                     "Color"]
        width = self.layer_model["config"]["obstacles"]["penWidth"]
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
                self.layer_model["config"]["transformation"]["key"],
                self.transformation_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["obstacles"]["key"],
                self.identifier)
