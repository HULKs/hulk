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
    name = "selfPlayer"

    def __init__(self, layer_model: ty.Dict, nao: Nao):
        merged_model = ui_utils.load_model(os.path.dirname(__file__) +
                                           "/model.json", layer_model)
        super(Main, self).__init__(merged_model, nao, str(uuid.uuid4()))

        self.pose = None
        self.role = None
        self.role_identifier = uuid.uuid4()
        self.jointSensorData = None
        self.jointSensorData_identifier = uuid.uuid4()
        self.motionPlan = None
        self.motionPlan_identifier = uuid.uuid4()
        self.searchPosition = None
        self.searchPosition_identifier = uuid.uuid4()
        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: Nao):
        self.nao = nao
        self.subscribe()

    def update_pose(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]["pose"]["keyLambda"], scope)
        self.pose = scope["output"]

    def update_role(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]["pose"]["roleKeyLambda"], scope)
        self.role = scope["output"]

    def update_jointSensorData(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]["fov"]["jointSensorDataKeyLambda"],
             scope)
        self.jointSensorData = scope["output"]

    def update_motionPlan(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]["motionPlan"]["keyLambda"], scope)
        self.motionPlan = scope["output"]

    def update_searchPosition(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]["ballSearch"]["keyLambda"], scope)
        self.searchPosition = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["pose"]["key"],
            self.identifier,
            lambda i: self.update_pose(i))
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["pose"]["roleKey"],
            self.role_identifier,
            lambda i: self.update_role(i))
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["fov"]["jointSensorDataKey"],
            self.jointSensorData_identifier,
            lambda i: self.update_jointSensorData(i))
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["motionPlan"]["key"],
            self.motionPlan_identifier,
            lambda i: self.update_motionPlan(i))
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["ballSearch"]["key"],
            self.searchPosition_identifier,
            lambda i: self.update_searchPosition(i))

    def paint(self, painter: Painter):
        if self.pose is not None:
            color = qtg.QColor(self.layer_model["config"
                                                ]["pose"
                                                  ]["fixedColor"])
            if not self.layer_model["config"]["pose"]["useFixedColor"]:
                if self.role is not None:
                    color = qtg.QColor(
                        self.layer_model["config"
                                         ]["pose"
                                           ][Roles(self.role).name + "Color"])
        if self.searchPosition is not None and self.pose is not None:
            if self.layer_model["config"]["ballSearch"]["drawSearchTarget"]:
                # Search position
                painter.setPen(qtg.QPen(qtc.Qt.black, 0))
                painter.setBrush(color)
                painter.drawEllipse(
                        qtc.QPointF(
                            self.searchPosition[0],
                            self.searchPosition[1]),
                        self.layer_model["config"
                                         ]["ballSearch"
                                           ]["searchCircleDiameter"] / 2,
                        self.layer_model["config"
                                         ]["ballSearch"
                                           ]["searchCircleDiameter"] / 2)
        if self.pose is not None:
            # Transform to local coords
            painter.transformByPose(self.pose)
        if self.jointSensorData is not None and self.pose is not None:
            if self.layer_model["config"]["fov"]["drawFOV"]:
                # FOV
                painter.setBrush(qtc.Qt.NoBrush)
                painter.setPen(qtg.QPen(qtc.Qt.yellow, 0))
                painter.drawFOV(
                    [[0.0, 0.0], 0.0], self.jointSensorData[0],
                    self.layer_model["config"]["fov"]["maxDistance"],
                    self.layer_model["config"]["fov"]["cameraOpeningAngle"])
        if self.motionPlan is not None and self.pose is not None:
            if self.layer_model["config"]["motionPlan"]["drawMotionPlan"]:
                # Walking-target
                painter.setBrush(qtc.Qt.NoBrush)
                painter.setPen(color, 0)
                painter.drawTarget(
                    self.motionPlan["walkTarget"],
                    self.layer_model["config"
                                     ]["motionPlan"
                                       ]["targetCircleDiameter"])
                # Dotted line to walking-target
                dotted_pen = qtg.QPen(qtc.Qt.yellow, 0, qtc.Qt.DashDotLine)
                painter.setPen(dotted_pen)
                painter.drawLineF(0.0, 0.0,
                                  self.motionPlan["walkTarget"][0][0],
                                  self.motionPlan["walkTarget"][0][1])
            if self.layer_model["config"]["motionPlan"]["drawTranslation"]:
                # Translation
                tl = self.motionPlan["translation"]
                painter.setPen(
                    qtg.QColor(self.layer_model["config"]["motionPlan"][
                        "translationColor"]), 0)
                orthoOffset = [tl[1] * 0.05, -tl[0] * 0.05]
                painter.drawLineF(0.0, 0.0, tl[0], tl[1])
                painter.drawLineF(tl[0] * 0.9 - orthoOffset[0],
                                  tl[1] * 0.9 - orthoOffset[1], tl[0], tl[1])
                painter.drawLineF(tl[0] * 0.9 + orthoOffset[0],
                                  tl[1] * 0.9 + orthoOffset[1], tl[0], tl[1])
        if self.pose is not None:
            if self.layer_model["config"]["pose"]["drawPose"]:
                # Pose
                painter.setPen(qtg.QPen(qtc.Qt.black, 0))
                painter.setBrush(color)
                painter.drawPose(
                    [[0.0, 0.0], 0.0],
                    self.layer_model["config"
                                     ]["pose"
                                       ]["positionCircleDiameter"],
                    self.layer_model["config"
                                     ]["pose"
                                       ]["orientationLineLength"])

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["pose"]["key"],
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["pose"]["roleKey"],
                self.role_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["fov"]["jointSensorDataKey"],
                self.jointSensorData_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["motionPlan"]["key"],
                self.motionPlan_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["ballSearch"]["key"],
                self.searchPosition_identifier)
