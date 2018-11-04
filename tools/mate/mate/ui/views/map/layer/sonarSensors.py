import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import uuid

from mate.ui.views.map.map_painter import Painter
from mate.ui.views.map.layer.layer import Layer
import mate.net.nao as nao


class SonarSensors(Layer):
    def __init__(self, layer: ty.Dict, nao: nao.Nao):
        self.nao = nao
        self.layer = layer
        self.identifier = uuid.uuid4()
        self.rawSonar = None
        self.filteredSonar = None
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

    def update_raw(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["sonar"]["rawKey_lambda"], scope)
        self.rawSonar = scope["output"]

    def update_filtered(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["sonar"]["filteredKey_lambda"], scope)
        self.filteredSonar = scope["output"]

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["transformation"]["key"],
            self.identifier,
            lambda i: self.update_transformation(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["sonar"]["rawKey"],
            self.identifier,
            lambda i: self.update_raw(i)
        )
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["sonar"]["filteredKey"],
            self.identifier,
            lambda i: self.update_filtered(i)
        )

    def paint(self, painter: Painter):
        painter.transformByPose(self.transformation)
        if self.filteredSonar is not None:
            sensors = [[-self.layer["settings"]["sonar"]["zAngle"],
                        self.layer["settings"]["sonar"]["yOffset"]/100,
                        self.filteredSonar["filteredValues"][0],
                        self.filteredSonar["valid"][0]],
                       [self.layer["settings"]["sonar"]["zAngle"],
                        -self.layer["settings"]["sonar"]["yOffset"]/100,
                        self.filteredSonar["filteredValues"][1],
                        self.filteredSonar["valid"][1]]]
            openAngle = self.layer["settings"]["sonar"]["openingAngle"]
            color = qtg.QColor(self.layer["settings"]["sonar"]["color"])
            for [zAngle, yOffset, filtered, fValid] in sensors:
                if fValid:
                    painter.setPen(color, 0.02)
                    rect = qtc.QRectF(0.0, 0.0, filtered*2, filtered*2)
                    rect.moveCenter(qtc.QPointF(0.0, yOffset))
                    painter.drawArc(rect,
                                    (-(openAngle/2)+zAngle)*16,
                                    openAngle*16)
                else:
                    painter.setPen(qtc.Qt.red, 0.02)
                    rect = qtc.QRectF(0.0, 0.0, 0.1, 0.1)
                    rect.moveCenter(qtc.QPointF(0.0, yOffset))
                    painter.drawPie(rect,
                                    (-(openAngle/2)+zAngle)*16,
                                    openAngle*16)
        if self.rawSonar is not None:
            sensors = [[-self.layer["settings"]["sonar"]["zAngle"],
                        self.layer["settings"]["sonar"]["yOffset"]/100,
                        self.rawSonar["SONAR_LEFT_SENSOR_0"],
                        self.rawSonar["valid_SONAR_LEFT_SENSOR_0"]],
                       [self.layer["settings"]["sonar"]["zAngle"],
                        -self.layer["settings"]["sonar"]["yOffset"]/100,
                        self.rawSonar["SONAR_RIGHT_SENSOR_0"],
                        self.rawSonar["valid_SONAR_RIGHT_SENSOR_0"]]]
            openAngle = self.layer["settings"]["sonar"]["openingAngle"]
            color = qtg.QColor(self.layer["settings"]["sonar"]["color"])
            for [zAngle, yOffset, raw, rValid] in sensors:
                if rValid:
                    painter.setPen(color, 0)
                    rect = qtc.QRectF(0.0, 0.0, raw*2, raw*2)
                    rect.moveCenter(qtc.QPointF(0.0, yOffset))
                    painter.drawArc(rect,
                                    (-(openAngle/2)+zAngle)*16,
                                    openAngle*16)
                else:
                    painter.setPen(qtc.Qt.red, 0)
                    rect = qtc.QRectF(0.0, 0.0, 0.1, 0.1)
                    rect.moveCenter(qtc.QPointF(0.0, yOffset))
                    painter.drawPie(rect,
                                    (-(openAngle/2)+zAngle)*16,
                                    openAngle*16)        

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["transformation"]["key"],
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["sonar"]["rawKey"],
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["sonar"]["filteredKey"],
                self.identifier)
