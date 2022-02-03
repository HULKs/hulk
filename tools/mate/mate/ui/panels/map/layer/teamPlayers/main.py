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


class Roles(enum.Enum):
    default = 0
    keeper = 1
    defender = 2
    supporter = 3
    striker = 4
    bishop = 5
    replacement_keeper = 6
    loser = 7
    searcher = 8


class Main(_Layer):
    name = "teamPlayers"

    def __init__(self, layer_model: ty.Dict, nao: Nao):
        merged_model = ui_utils.load_model(os.path.dirname(__file__) +
                                           "/model.json", layer_model)
        super(Main, self).__init__(merged_model, nao, str(uuid.uuid4()))

        self.teamPlayers = None

        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: Nao):
        self.nao = nao
        self.subscribe()

    def update_team_players(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]["teamPlayers"]["keyLambda"],
             scope)
        self.teamPlayers = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["teamPlayers"]["key"],
            self.identifier, lambda i: self.update_team_players(i))

    def paint(self, painter: Painter):
        if self.teamPlayers is not None:
            # Draw for each team player
            for robot in self.teamPlayers:
                color = qtg.QColor(
                    self.layer_model["config"]["teamPlayers"][Roles(
                        robot["currentlyPerformingRole"]).name + "Color"])
                # fov
                if self.layer_model["config"]["teamPlayers"]["showFOV"]:
                    # fov
                    painter.setBrush(qtc.Qt.NoBrush)
                    painter.setPen(qtg.QPen(qtc.Qt.yellow, 0))
                    painter.drawFOV(robot["pose"],
                                    robot["headYaw"],
                                    self.layer_model["config"]["teamPlayers"]
                                    ["maxDistance"],
                                    self.layer_model["config"]["teamPlayers"]
                                    ["cameraOpeningAngle"])
                if self.layer_model["config"]["teamPlayers"]["showTarget"]:
                    # Walking-target
                    painter.setBrush(qtc.Qt.NoBrush)
                    painter.setPen(color, 0)
                    painter.drawTarget(
                        robot["walkingTo"],
                        self.layer_model["config"
                                         ]["teamPlayers"
                                           ]["targetCircleDiameter"])
                    # Dotted line
                    dotted_pen = qtg.QPen(qtc.Qt.yellow,
                                          0,
                                          qtc.Qt.DashDotLine)
                    painter.setPen(dotted_pen)
                    painter.drawLineF(robot["pose"][0][0],
                                      robot["pose"][0][1],
                                      robot["walkingTo"][0][0],
                                      robot["walkingTo"][0][1])
                if self.layer_model["config"]["teamPlayers"]["showPlayer"]:
                    # Pose
                    painter.setBrush(color)
                    painter.setPen(qtg.QPen(qtc.Qt.black, 0))
                    painter.drawPose(
                        robot["pose"],
                        self.layer_model["config"
                                         ]["teamPlayers"
                                           ]["poseCircleDiameter"],
                        self.layer_model["config"
                                         ]["teamPlayers"
                                           ]["poseCircleDiameter"])
                    # PlayerNumber
                    painter.setPen(
                        qtg.QPen(qtg.QColor(ui_utils.ideal_text_color(color)),
                                 0))
                    diameter = self.layer_model["config"
                                                ]["teamPlayers"
                                                  ]["poseCircleDiameter"]
                    painter.drawText(
                        qtc.QPointF(
                            robot["pose"][0][0] - (diameter * 0.2),
                            robot["pose"][0][1] - (diameter * 0.25)),
                        str(robot["playerNumber"]),
                        diameter * 0.61)
                if self.layer_model["config"
                                    ]["teamPlayers"
                                      ]["showSearchPosition"]:
                    # Show Search Position
                    painter.setBrush(color)
                    painter.setPen(qtg.QPen(qtc.Qt.black, 0))
                    painter.drawEllipse(
                        qtc.QPointF(
                            robot["currentSearchPosition"][0],
                            robot["currentSearchPosition"][1]),
                        self.layer_model["config"
                                         ]["teamPlayers"
                                           ]["searchPositionDiameter"] / 2,
                        self.layer_model["config"
                                         ]["teamPlayers"
                                           ]["searchPositionDiameter"] / 2)

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["teamPlayers"]["key"],
                self.identifier)
