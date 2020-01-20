import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import uuid
import os
import math
import numpy as np

from mate.ui.panels.map.map_painter import Painter
from mate.ui.panels.map.layer._layer_main import _Layer
from mate.net.nao import Nao
import mate.ui.utils as ui_utils


class Main(_Layer):
    name = "robotsFilter"

    def __init__(self, layer_model: ty.Dict, nao: Nao):
        merged_model = ui_utils.load_model(os.path.dirname(__file__) +
                                           "/model.json", layer_model)
        super(Main, self).__init__(merged_model, nao, str(uuid.uuid4()))

        self.robots = None

        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: Nao):
        self.nao = nao
        self.subscribe()

    def update_robots_data(self, data):
        self.robots = data.data

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["robotsFilter"]["key"], self.identifier,
            lambda i: self.update_robots_data(i))

    def paint(self, painter: Painter):
        if self.robots is None:
            return

        painter.setPen(qtg.QPen(qtc.Qt.black, 0))
        painter.setBrush(qtc.Qt.blue)

        for robot in self.robots:
            self.drawRobot(painter, robot)

    def drawRobot(self, painter: Painter, robot):
        painter.setBrush(qtg.QColor(255, 255, 0, 120))

        cov = np.array(robot["covariance"])
        w, v = np.linalg.eig(cov)
        v1 = v[:, 0]
        phi_inferred = np.rad2deg(np.arctan2(v1[1], v1[0]))

        std_x = math.sqrt(w[0])
        std_y = math.sqrt(w[1])

        painter.save()
        painter.translate(robot["state"][0], robot["state"][1])
        painter.rotate(-phi_inferred)
        painter.drawEllipse(qtc.QPointF(0, 0), std_x, std_y)
        painter.restore()
        painter.setBrush(qtc.Qt.red)
        painter.drawEllipse(
            qtc.QPointF(robot["state"][0], robot["state"][1]),
            0.1, 0.1
        )

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["robotsFilter"]["key"], self.identifier)
