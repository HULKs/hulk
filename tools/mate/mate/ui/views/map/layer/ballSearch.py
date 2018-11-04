import typing as ty
import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg

import uuid

import math

from mate.ui.views.map.map_painter import Painter
from mate.ui.views.map.layer.layer import Layer
import mate.net.nao as nao


class BallSearch(Layer):
    def __init__(self, layer: ty.Dict, nao: nao.Nao):
        self.nao = nao
        self.layer = layer
        self.identifier = uuid.uuid4()
        self.probabilityMap = None
        self.explorerCount = 0
        self.voronoiSeeds = None
        self.ballSearchPose = None
        self.walkTarget = None
        self.pose = None
        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: nao.Nao):
        self.nao = nao
        self.subscribe()

    def update_probabilityMap(self, data):
        scope = {"input": data.data, "output": None}
        exec(self.layer["settings"]["search"]["keyLambda"], scope)
        self.probabilityMap = scope["output"]["probabilityMap"]

    def update_voronoiSeeds(self, data):
        self.voronoiSeeds = data.data

    def update_ballSearchPose(self, data):
        self.ballSearchPose = data.data["pose"]

    def update_walkTarget(self, data):
        self.walkTarget = data.data["walkTarget"]

    def update_pose(self, data):
        self.pose = data.data["pose"]

    def update_explorerCount(self, data):
        self.explorerCount = data.data

    def subscribe(self):
        self.nao.debug_protocol.subscribe(
            self.layer["settings"]["search"]["key"],
            self.identifier,
            lambda i: self.update_probabilityMap(i)
        )
        self.nao.debug_protocol.subscribe(
            "Brain.BallSearchPositionProvider.voronoiSeeds",
            self.identifier,
            lambda i: self.update_voronoiSeeds(i)
        )
        self.nao.debug_protocol.subscribe(
            "Brain.BallSearchPosition",
            self.identifier,
            lambda i: self.update_ballSearchPose(i)
        )
        self.nao.debug_protocol.subscribe(
            "Brain.MotionPlanner",
            self.identifier,
            lambda i: self.update_walkTarget(i)
        )
        self.nao.debug_protocol.subscribe(
            "Brain.RobotPosition",
            self.identifier,
            lambda i: self.update_pose(i)
        )
        self.nao.debug_protocol.subscribe(
            "Brain.BallSearchPositionProvider.explorerCount",
            self.identifier,
            lambda i: self.update_explorerCount(i)
        )

    def paint(self, painter: Painter):
        if self.probabilityMap:
            paintText = False
            if self.layer["settings"]["search"]["showNumericProbability"]:
                paintText = True
            elif self.layer["settings"]["search"]["showNumericAge"]:
                paintText = True
            for row in self.probabilityMap:
                for cell in row:
                    painter.setPen(qtc.Qt.black, 0.0)
                    # to help debugging
                    if (cell[0] < 0.0):
                        print("cell probability < 0 p:", cell[0])
                    if (cell[0] > 1.0):
                        print("cell probability > 1 p:", cell[0])
                    if (cell[1] < 0):
                        print("cell age < 0 age:", cell[1])
                    # Age in redscale as outline, saturated at 50 seconds
                    if self.layer["settings"]["search"]["showAge"]:
                        painter.setPen(
                            qtg.QColor(round(
                                255 * ((min(cell[1], 5000)) / 5000)),
                                       0,
                                       0,
                                       255),
                            0.02)
                    # Probability in greyscale at center,
                    # scaled by fourth root (^0.25) for visibility
                    scaledProbability = 0
                    if self.layer["settings"]["search"]["showProbability"]:
                        scaledProbability = round(255 *
                                                  (cell[0]**0.25))
                        scaledProbability = max(0, 
                                                min(255, scaledProbability)) 
                    painter.setBrush(
                        qtg.QColor(
                            scaledProbability,
                            scaledProbability,
                            scaledProbability,
                            255))
                    painter.drawRect(
                        qtc.QRectF(
                            cell[2] - 0.18,
                            cell[3] - 0.18,
                            0.36,
                            0.36))
                    if paintText:
                        # Showing numeric values
                        painter.setPen(qtc.Qt.white, 0)
                        # Probability in %
                        if self.layer["settings"
                                      ]["search"
                                        ]["showNumericProbability"]:
                            painter.drawText(
                                qtc.QPointF(
                                    cell[2] - 0.15,
                                    cell[3] + 0.03),
                                "%.2f" % (cell[0] * 100.0),
                                0.01)
                        # Age in seconds
                        if self.layer["settings"]["search"]["showNumericAge"]:
                            ageToShow = math.floor(cell[1] * 0.016667)
                            logOffset = 0
                            if ageToShow > 0:
                                logOffset = math.floor(math.log10(ageToShow))
                            painter.drawText(
                                qtc.QPointF(
                                    cell[2] - 0.025 -
                                    (0.05 * logOffset),
                                    cell[3] - 0.15),
                                str(ageToShow),
                                0.01)
        # Show voronoi seeds: opaque when searching, transparent when not
        if self.voronoiSeeds and (self.explorerCount > 0):
            if self.layer["settings"]["search"]["showVoronoiSeeds"]:
                # To check if search is active,
                # we need walkTarget in absolute coordinates and
                # compare it to ballSearchPose.
                # This is a very ugly method,
                # but very useful information for testing.
                searching = 128  # not searching actively
                if self.ballSearchPose:
                    if self.walkTarget:
                        if self.pose:
                            walkTargetAbsolute = [
                                [self.pose[0][0] +
                                 (math.cos(self.pose[1]) *
                                  self.walkTarget[0][0]) -
                                 (math.sin(self.pose[1]) *
                                  self.walkTarget[0][1]),
                                 self.pose[0][1] +
                                 (math.sin(self.pose[1]) *
                                  self.walkTarget[0][0]) +
                                 (math.cos(self.pose[1]) *
                                  self.walkTarget[0][1])],
                                (self.pose[1] +
                                 self.walkTarget[1]) %
                                (math.pi * 2.0)]
                            if walkTargetAbsolute[1] > math.pi:
                                walkTargetAbsolute[1] -= math.pi * 2.0
                            elif walkTargetAbsolute[1] < -math.pi:
                                walkTargetAbsolute[1] += math.pi * 2.0
                            # For the comparison a high threshold is being
                            # used to avoid flickering,
                            # as data might be somewhat out of sync
                            if abs(walkTargetAbsolute[1] -
                                   self.ballSearchPose[1]) < 0.1:
                                if abs(walkTargetAbsolute[0][0] -
                                       self.ballSearchPose[0][0]) < 0.1:
                                    if abs(walkTargetAbsolute[0][1] -
                                           self.ballSearchPose[0][1]) < 0.1:
                                        searching = 255  # actively searching
                # paint voronoi seeds
                painter.setPen(qtc.Qt.black, 0)
                painter.setBrush(qtg.QColor(255, 255, 0, searching))
                for seed in self.voronoiSeeds:
                    painter.drawEllipse(
                        qtc.QPointF(seed[0], seed[1]),
                        0.15,
                        0.15)

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.layer["settings"]["search"]["key"],
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                "Brain.BallSearchPositionProvider.voronoiSeeds",
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                "Brain.BallSearchPosition",
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                "Brain.MotionPlanner",
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                "Brain.RobotPosition",
                self.identifier)
            self.nao.debug_protocol.unsubscribe(
                "Brain.BallSearchPositionProvider.explorerCount",
                self.identifier)
