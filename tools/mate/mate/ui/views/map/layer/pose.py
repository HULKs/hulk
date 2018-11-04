import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import uuid
import math

from mate.ui.views.map.map_painter import Painter
from mate.ui.views.map.layer.layer import Layer
import mate.net.nao as nao


class Pose(Layer):
    def __init__(self, layer: ty.Dict, nao: nao.Nao):
        self.nao = nao
        self.layer = layer
        self.identifier = uuid.uuid4()
        self.pose = None
        self.transformation = [[0, 0], 0]
        self.joint_sensor_data = None
        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: nao.Nao):
        self.nao = nao
        self.subscribe()

    def update_pose(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["pose"]["keyLambda"], scope)
        self.pose = scope["output"]

    def update_transformation(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["transformation"]["key_lambda"], scope)
        self.transformation = scope["output"]

    def update_jointSensorData(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["fov"]["jointSensorDataKeyLambda"], scope)
        self.joint_sensor_data = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["transformation"]["key"],
            self.identifier,
            lambda i: self.update_transformation(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["pose"]["key"],
            self.identifier,
            lambda i: self.update_pose(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["fov"]["jointSensorDataKey"],
            self.identifier,
            lambda i: self.update_jointSensorData(i))

    def paint(self, painter: Painter):
        painter.transformByPose(self.transformation)
        painter.setPen(qtg.QPen(qtc.Qt.black, 0))
        painter.setBrush(qtg.QColor(
            self.layer["settings"]["pose"]["color"]))

        # blue pose indicator
        if self.pose is not None:
            painter.drawPose(
                self.pose,
                self.layer["settings"]["pose"]["positionCircleDiameter"],
                self.layer["settings"]["pose"]["orientationLineLength"])

        # fov
        if self.joint_sensor_data is not None and self.pose is not None:
            painter.setBrush(qtc.Qt.NoBrush)
            painter.setPen(qtg.QPen(qtc.Qt.yellow, 0))
            painter.drawFOV(self.pose,
                            self.joint_sensor_data[0],
                            self.layer["settings"
                                       ]["fov"
                                         ]["maxDistance"],
                            self.layer["settings"
                                       ]["fov"
                                         ]["cameraOpeningAngle"])

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["transformation"]["key"],
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["pose"]["key"],
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["fov"]["jointSensorDataKey"],
                self.identifier)
