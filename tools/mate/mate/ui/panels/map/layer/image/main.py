import typing as ty
import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc

import uuid
import os
import math
import numpy as np

import cv2

from mate.ui.panels.map.map_painter import Painter
from mate.ui.panels.map.layer._layer_main import _Layer
from mate.net.nao import Nao
import mate.ui.utils as ui_utils
from mate.debug.colorlog import ColorLog


class Main(_Layer):
    name = "image"

    def __init__(self, layer_model: ty.Dict, nao: Nao):
        merged_model = ui_utils.load_model(os.path.dirname(__file__) +
                                           "/model.json", layer_model)
        super(Main, self).__init__(merged_model, nao, str(uuid.uuid4()))

        self.ignore_perspective = True
        self.ignore_scale = True

        self.pixmap = qtg.QPixmap()
        self.image = qtg.QImage()

        if self.config["mode"] == "File":
            self.image.load(self.config["filepath"])

        if self.config["mode"] == "Debug Key":
            if self.nao.is_connected():
                self.connect(self.nao)
            self.data = None
            self.should_update = True

        if self.config["mode"] == "Video Device":
            self.vc = cv2.VideoCapture(self.config["video_device"])

    def connect(self, nao: Nao):
        self.nao = nao
        self.subscribe(self.config["subscribe_key"])

    def data_received(self, data):
        self.data = data
        self.should_update = True

    def subscribe(self, key):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.config["subscribe_key"],
                self.identifier)
            self.nao.debug_protocol.subscribe(
                key,
                self.identifier,
                lambda d: self.data_received(d))
        self.config["subscribe_key"] = key

    def unsubscribe(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(self.config["subscribe_key"],
                                                self.identifier)

    def update_image(self):
        # nothing to do fo static files
        if self.config["mode"] == "Debug Key" and self.data is not None and self.should_update:
            self.image.loadFromData(self.data.data)
            self.should_update = False
        if self.config["mode"] == "Video Device":
            rval, frame = self.vc.read()
            if not rval:
                logger = ColorLog()
                logger.error("Failed to read frame from camera device")
                return
            frame = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
            self.image = qtg.QImage(
                frame, frame.shape[1], frame.shape[0], qtg.QImage.Format_RGB888)

    def paint(self, painter: Painter):
        self.update_image()
        painter.scale(1, -1)
        rect = qtc.QRectF(
            -1.0/2.0,  # xpos
            -1.0/2.0,  # ypos
            1,  # width
            1  # height
        )
        painter.drawImage(rect, self.image)

    def destroy(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(
                self.config["subscribe_key"], self.identifier)
