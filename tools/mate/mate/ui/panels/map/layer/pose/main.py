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
    name = "Pose"

    def __init__(self, layer_model: ty.Dict, nao: Nao):
        merged_model = ui_utils.load_model(os.path.dirname(__file__) +
                                           "/model.json", layer_model)
        super(Main, self).__init__(merged_model, nao, str(uuid.uuid4()))

        self.pose = None
        self.transformation = [[0, 0], 0]
        self.transformation_identifier = uuid.uuid4()
        self.joint_sensor_data = None
        self.joint_sensor_data_identifier = uuid.uuid4()
        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: Nao):
        self.nao = nao
        self.subscribe()

    def update_pose(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]["pose"]["keyLambda"], scope)
        self.pose = scope["output"]

    def update_transformation(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]["transformation"]["key_lambda"], scope)
        self.transformation = scope["output"]

    def update_jointSensorData(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer_model["config"]["fov"]["jointSensorDataKeyLambda"],
             scope)
        self.joint_sensor_data = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["transformation"]["key"],
            self.transformation_identifier,
            lambda i: self.update_transformation(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["pose"]["key"],
            self.identifier,
            lambda i: self.update_pose(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["fov"]["jointSensorDataKey"],
            self.joint_sensor_data_identifier,
            lambda i: self.update_jointSensorData(i))

    def paint(self, painter: Painter):
        painter.transformByPose(self.transformation)
        painter.setPen(qtg.QPen(qtc.Qt.black, 0))
        painter.setBrush(qtg.QColor(
            self.layer_model["config"]["pose"]["color"]))

        # blue pose indicator
        if self.pose is not None:
            painter.drawPose(
                self.pose,
                self.layer_model["config"]["pose"]["positionCircleDiameter"],
                self.layer_model["config"]["pose"]["orientationLineLength"])

        # fov
        if self.joint_sensor_data is not None and self.pose is not None:
            painter.setBrush(qtc.Qt.NoBrush)
            painter.setPen(qtg.QPen(qtc.Qt.yellow, 0))
            painter.drawFOV(self.pose,
                            self.joint_sensor_data[0],
                            self.layer_model["config"
                                             ]["fov"
                                               ]["maxDistance"],
                            self.layer_model["config"
                                             ]["fov"
                                               ]["cameraOpeningAngle"])

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["transformation"]["key"],
                self.transformation_identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["pose"]["key"],
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["fov"]["jointSensorDataKey"],
                self.joint_sensor_data_identifier)
