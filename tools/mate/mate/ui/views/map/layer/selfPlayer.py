import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc
import uuid
import enum

from mate.ui.views.map.map_painter import Painter
from mate.ui.views.map.layer.layer import Layer
import mate.net.nao as nao


class Roles(enum.Enum):
    default = 0
    keeper = 1
    defender = 2
    supporter = 3
    striker = 4
    bishop = 5
    replacement_keeper = 6


class SelfPlayer(Layer):
    def __init__(self, layer: ty.Dict, nao: nao.Nao):
        self.nao = nao
        self.layer = layer
        self.identifier = uuid.uuid4()
        self.pose = None
        self.role = None
        self.jointSensorData = None
        self.motionPlan = None
        self.searchPosition = None
        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: nao.Nao):
        self.nao = nao
        self.subscribe()

    def update_pose(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["pose"]["keyLambda"], scope)
        self.pose = scope["output"]

    def update_role(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["pose"]["roleKeyLambda"], scope)
        self.role = scope["output"]

    def update_jointSensorData(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["fov"]["jointSensorDataKeyLambda"], scope)
        self.jointSensorData = scope["output"]

    def update_motionPlan(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["motionPlan"]["keyLambda"], scope)
        self.motionPlan = scope["output"]

    def update_searchPosition(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["ballSearch"]["keyLambda"], scope)
        self.searchPosition = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["pose"]["key"], self.identifier,
            lambda i: self.update_pose(i))
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["pose"]["roleKey"], self.identifier,
            lambda i: self.update_role(i))
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["fov"]["jointSensorDataKey"],
            self.identifier, lambda i: self.update_jointSensorData(i))
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["motionPlan"]["key"], self.identifier,
            lambda i: self.update_motionPlan(i))
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["ballSearch"]["key"], self.identifier,
            lambda i: self.update_searchPosition(i))

    def paint(self, painter: Painter):
        if self.pose is not None:
            color = qtg.QColor(self.layer["settings"]["pose"]["fixedColor"])
            if not self.layer["settings"]["pose"]["useFixedColor"]:
                if self.role is not None:
                    color = qtg.QColor(
                        self.layer["settings"
                                   ]["pose"
                                     ][Roles(self.role).name + "Color"])
        if self.searchPosition is not None and self.pose is not None:
            if self.layer["settings"]["ballSearch"]["drawSearchTarget"]:
                # Search position
                painter.setPen(qtg.QPen(qtc.Qt.black, 0))
                painter.setBrush(color)
                painter.drawEllipse(
                        qtc.QPointF(
                            self.searchPosition[0],
                            self.searchPosition[1]),
                        self.layer["settings"
                                   ]["ballSearch"
                                     ]["searchCircleDiameter"] / 2,
                        self.layer["settings"
                                   ]["ballSearch"
                                     ]["searchCircleDiameter"] / 2)
        if self.pose is not None:
            # Transform to local coords
            painter.transformByPose(self.pose)
        if self.jointSensorData is not None and self.pose is not None:
            if self.layer["settings"]["fov"]["drawFOV"]:
                # FOV
                painter.setBrush(qtc.Qt.NoBrush)
                painter.setPen(qtg.QPen(qtc.Qt.yellow, 0))
                painter.drawFOV(
                    [[0.0, 0.0], 0.0], self.jointSensorData[0],
                    self.layer["settings"]["fov"]["maxDistance"],
                    self.layer["settings"]["fov"]["cameraOpeningAngle"])
        if self.motionPlan is not None and self.pose is not None:
            if self.layer["settings"]["motionPlan"]["drawMotionPlan"]:
                # Walking-target
                painter.setBrush(qtc.Qt.NoBrush)
                painter.setPen(color, 0)
                painter.drawTarget(self.motionPlan["walkTarget"], self.layer[
                    "settings"]["motionPlan"]["targetCircleDiameter"])
                # Dotted line to walking-target
                dotted_pen = qtg.QPen(qtc.Qt.yellow, 0, qtc.Qt.DashDotLine)
                painter.setPen(dotted_pen)
                painter.drawLineF(0.0, 0.0,
                                  self.motionPlan["walkTarget"][0][0],
                                  self.motionPlan["walkTarget"][0][1])
            if self.layer["settings"]["motionPlan"]["drawTranslation"]:
                # Translation
                tl = self.motionPlan["translation"]
                painter.setPen(
                    qtg.QColor(self.layer["settings"]["motionPlan"][
                        "translationColor"]), 0)
                orthoOffset = [tl[1] * 0.05, -tl[0] * 0.05]
                painter.drawLineF(0.0, 0.0, tl[0], tl[1])
                painter.drawLineF(tl[0] * 0.9 - orthoOffset[0],
                                  tl[1] * 0.9 - orthoOffset[1], tl[0], tl[1])
                painter.drawLineF(tl[0] * 0.9 + orthoOffset[0],
                                  tl[1] * 0.9 + orthoOffset[1], tl[0], tl[1])
        if self.pose is not None:
            if self.layer["settings"]["pose"]["drawPose"]:
                # Pose
                painter.setPen(qtg.QPen(qtc.Qt.black, 0))
                painter.setBrush(color)
                painter.drawPose(
                    [[0.0, 0.0], 0.0],
                    self.layer["settings"]["pose"]["positionCircleDiameter"],
                    self.layer["settings"]["pose"]["orientationLineLength"])

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["pose"]["key"], self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["pose"]["roleKey"], self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["fov"]["jointSensorDataKey"],
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["motionPlan"]["key"], self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["ballSearch"]["key"], self.identifier)
