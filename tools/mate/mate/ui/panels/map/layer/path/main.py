import typing as ty
import math
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import uuid
import os

from mate.ui.panels.map.map_painter import Painter
from mate.ui.panels.map.layer._layer_main import _Layer
from mate.net.nao import Nao
import mate.ui.utils as ui_utils


class Main(_Layer):
    name = "path"

    def __init__(self, layer_model: ty.Dict, nao: Nao):
        merged_model = ui_utils.load_model(
            os.path.dirname(__file__) + "/model.json", layer_model)
        super(Main, self).__init__(merged_model, nao, str(uuid.uuid4()))

        self.pathLines = None
        self.arcLines = None
        self.pathObstacles = None
        self.nodes = None
        self.start_nodes = None
        self.end_nodes = None
        self.blockedArcLines = None

        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: Nao):
        self.nao = nao
        self.subscribe()

    def update_path_data(self, data):
        self.pathLines = list()
        self.arcLines = list()
        self.start_nodes = list()
        self.end_nodes = list()
        self.blockedArcLines = list()
        for path in data.data["path"]["edges"]:
            if isinstance(path, list):
                self.pathLines.append(path)
                self.start_nodes.append(path[0])
                self.end_nodes.append(path[1])
            elif isinstance(path, dict):
                arc = path
                x = arc['circle'][0][0] - arc['circle'][1]  # center.x - radius
                y = arc['circle'][0][1] - arc['circle'][1]  # center.y - radius
                self.start_nodes.append(arc['start'])
                self.end_nodes.append(arc['end'])
                diameter = arc['circle'][1] * 2.0
                start = int(
                    math.atan2(arc['start'][1] - arc['circle'][0][1],
                               arc['start'][0] - arc['circle'][0][0]) /
                    math.pi * 180)
                end = int(
                    math.atan2(arc['end'][1] - arc['circle'][0][1],
                               arc['end'][0] - arc['circle'][0][0]) / math.pi *
                    180)
                angleDiff = abs(end - start)
                if arc['clockwise']:
                    if end <= start:
                        angleDiff = -start + end
                    else:
                        angleDiff = -360 + end - start
                    start = -end
                else:
                    if start <= end:
                        angleDiff = start - end
                    else:
                        angleDiff = -360 - end + start
                    start = -start
                self.arcLines.append((x, y, diameter, start, angleDiff))

    def update_obstacle_data(self, data):
        self.pathObstacles = list()
        for obstacle in data.data:
            self.pathObstacles.append(obstacle)

        for obstacle in data.data:
            for arc in obstacle["blockedArcs"]:
                x = arc['circle'][0][0] - arc['circle'][1]  # center.x - radius
                y = arc['circle'][0][1] - arc['circle'][1]  # center.y - radius
                diameter = arc['circle'][1] * 2.0
                start = int(
                    math.atan2(arc['start'][1] - arc['circle'][0][1],
                               arc['start'][0] - arc['circle'][0][0]) /
                    math.pi * 180)
                end = int(
                    math.atan2(arc['end'][1] - arc['circle'][0][1],
                               arc['end'][0] - arc['circle'][0][0]) / math.pi *
                    180)
                angleDiff = abs(end - start)
                if arc['clockwise']:
                    if end <= start:
                        angleDiff = -start + end
                    else:
                        angleDiff = -360 + end - start
                    start = -end
                else:
                    if start <= end:
                        angleDiff = start - end
                    else:
                        angleDiff = -360 - end + start
                    start = -start
                self.blockedArcLines.append((x, y, diameter, start, angleDiff))

    def update_node_data(self, data):
        self.nodes = list()
        for node in data.data:
            self.nodes.append([
                node["absolutePosition"][0], node["absolutePosition"][1],
                node["nodeType"]
            ])

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["path"]["key"], self.identifier,
            lambda i: self.update_path_data(i))
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["pathObstacles"]["key"],
            self.identifier, lambda i: self.update_obstacle_data(i))
        self.nao.debug_protocol.subscribe(
            self.layer_model["config"]["pathNodes"]["key"], self.identifier,
            lambda i: self.update_node_data(i))

    def paint(self, painter: Painter):
        if self.pathObstacles:
            painter.setBrush(qtc.Qt.NoBrush)
            painter.setPen(qtg.QPen(qtc.Qt.blue, 0.03))
            for obstacle in self.pathObstacles:
                painter.drawEllipse(
                    qtc.QPointF(obstacle["circlePosition"][0],
                                obstacle["circlePosition"][1]),
                    obstacle["radius"], obstacle["radius"])

        if self.pathLines:
            painter.setBrush(qtc.Qt.NoBrush)
            painter.setPen(qtg.QPen(qtc.Qt.green, 0.1))
            for line in self.pathLines:
                painter.drawLineF(line[0][0], line[0][1], line[1][0],
                                  line[1][1])

        if self.arcLines:
            painter.setBrush(qtc.Qt.NoBrush)
            painter.setPen(qtg.QPen(qtc.Qt.green, 0.1))
            for arc in self.arcLines:
                x, y, diameter, start, angleDiff = arc
                painter.drawArc(qtc.QRectF(x, y, diameter, diameter),
                                start * 16, angleDiff * 16)

        if self.blockedArcLines:
            painter.setBrush(qtc.Qt.NoBrush)
            painter.setPen(qtg.QPen(qtc.Qt.red, 0.1))
            for arc in self.blockedArcLines:
                x, y, diameter, start, angleDiff = arc
                painter.drawArc(qtc.QRectF(x, y, diameter, diameter),
                                start * 16, angleDiff * 16)

        if self.nodes:
            painter.setBrush(qtc.Qt.NoBrush)
            painter.setPen(qtg.QPen(qtc.Qt.yellow, 0.04))
            for node in self.nodes:
                painter.drawPoint(qtc.QPointF(node[0], node[1]))

        if self.start_nodes:
            painter.setBrush(qtc.Qt.NoBrush)
            painter.setPen(qtg.QPen(qtc.Qt.black, 0.12))
            for node in self.start_nodes:
                painter.drawPoint(qtc.QPointF(node[0], node[1]))

        if self.end_nodes:
            painter.setBrush(qtc.Qt.NoBrush)
            painter.setPen(qtg.QPen(qtc.Qt.red, 0.06))
            for node in self.end_nodes:
                painter.drawPoint(qtc.QPointF(node[0], node[1]))

        # Draw special nodes
        if self.nodes:
            painter.setBrush(qtc.Qt.NoBrush)
            for node in self.nodes:
                if node[2] == 0:
                    continue
                if node[2] == 1:
                    painter.setPen(qtg.QPen(qtc.Qt.blue, 0.15))
                else:
                    painter.setPen(qtg.QPen(qtc.Qt.red, 0.15))
                painter.drawPoint(qtc.QPointF(node[0], node[1]))

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer_model["config"]["path"]["key"], self.identifier)
