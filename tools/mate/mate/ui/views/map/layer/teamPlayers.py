import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import uuid
import math
import enum

from mate.ui.views.map.map_painter import Painter
from mate.ui.views.map.layer.layer import Layer
import mate.net.nao as nao
import mate.ui.utils as ui_utils


class Roles(enum.Enum):
    default = 0
    keeper = 1
    defender = 2
    supporter = 3
    striker = 4
    bishop = 5
    replacement_keeper = 6


class TeamPlayers(Layer):
    def __init__(self, layer: ty.Dict, nao: nao.Nao):
        self.nao = nao
        self.layer = layer
        self.identifier = uuid.uuid4()
        self.teamPlayers = None
        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: nao.Nao):
        self.nao = nao
        self.subscribe()

    def update_team_players(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["teamPlayers"]["keyLambda"],
             scope)
        self.teamPlayers = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["teamPlayers"]["key"],
            self.identifier, lambda i: self.update_team_players(i))

    def paint(self, painter: Painter):
        if self.teamPlayers is not None:
            # Draw for each team player
            for robot in self.teamPlayers:
                color = qtg.QColor(
                    self.layer["settings"]["teamPlayers"][Roles(
                        robot["currentlyPerformingRole"]).name + "Color"])
                # fov
                if self.layer["settings"]["teamPlayers"]["showFOV"]:
                    # fov
                    painter.setBrush(qtc.Qt.NoBrush)
                    painter.setPen(qtg.QPen(qtc.Qt.yellow, 0))
                    painter.drawFOV(robot["pose"],
                                    robot["headYaw"],
                                    self.layer["settings"]["teamPlayers"]
                                    ["maxDistance"],
                                    self.layer["settings"]["teamPlayers"]
                                    ["cameraOpeningAngle"])
                if self.layer["settings"]["teamPlayers"]["showTarget"]:
                    # Walking-target
                    painter.setBrush(qtc.Qt.NoBrush)
                    painter.setPen(color, 0)
                    painter.drawTarget(robot["walkingTo"],
                                       self.layer["settings"]["teamPlayers"]
                                       ["targetCircleDiameter"])
                    # Dotted line
                    dotted_pen = qtg.QPen(qtc.Qt.yellow,
                                          0,
                                          qtc.Qt.DashDotLine)
                    painter.setPen(dotted_pen)
                    painter.drawLineF(robot["pose"][0][0],
                                      robot["pose"][0][1],
                                      robot["walkingTo"][0][0],
                                      robot["walkingTo"][0][1])
                if self.layer["settings"]["teamPlayers"]["showPlayer"]:
                    # Pose
                    painter.setBrush(color)
                    painter.setPen(qtg.QPen(qtc.Qt.black, 0))
                    painter.drawPose(robot["pose"],
                                    self.layer["settings"]["teamPlayers"]
                                    ["poseCircleDiameter"],
                                    self.layer["settings"]["teamPlayers"]
                                    ["poseCircleDiameter"])
                    # PlayerNumber
                    painter.setPen(
                        qtg.QPen(qtg.QColor(ui_utils.ideal_text_color(color)),
                                 0))
                    painter.drawText(qtc.QPointF(robot["pose"][0][0] -
                                             (self.layer["settings"]
                                              ["teamPlayers"]
                                              ["poseCircleDiameter"] * 0.2),
                                             robot["pose"][0][1] -
                                             (self.layer["settings"]
                                              ["teamPlayers"]
                                              ["poseCircleDiameter"] * 0.25)),
                                 str(robot["playerNumber"]),
                                 self.layer["settings"]
                                 ["teamPlayers"]["poseCircleDiameter"] * 0.61)
                if self.layer["settings"]["teamPlayers"]["showSearchPosition"]:
                    # Show Search Position
                    painter.setBrush(color)
                    painter.setPen(qtg.QPen(qtc.Qt.black, 0))
                    painter.drawEllipse(
                        qtc.QPointF(
                            robot["currentSearchPosition"][0],
                            robot["currentSearchPosition"][1]),
                        self.layer["settings"]["teamPlayers"]["searchPositionDiameter"] / 2,
                        self.layer["settings"]["teamPlayers"]["searchPositionDiameter"] / 2)
                    

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["teamPlayers"]["key"],
                self.identifier)
