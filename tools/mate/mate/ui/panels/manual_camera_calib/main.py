import json
import os
import typing

import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg
import PyQt5.QtWidgets as qtw

import mate.net.nao_data as nd
import mate.ui.utils as ui_utils
import mate.net.utils as net_utils
from mate.net.nao import Nao
from mate.ui.panels._panel import _Panel
from mate.debug.colorlog import ColorLog

logger = ColorLog()


class Main(_Panel):
    name = "ManualCamConfig"
    shortcut = qtg.QKeySequence("Ctrl+E")

    update_signal = qtc.pyqtSignal(nd.ConfigMount)

    def __init__(self, main_window, nao: Nao, model: typing.Dict = None):
        super(Main, self).__init__(main_window, self.name, nao)
        ui_utils.loadUi(__file__, self)
        self.model = ui_utils.load_model(os.path.dirname(__file__) +
                                         "/model.json", model)
        self.data = None
        self.btnSetAll.clicked.connect(self.set)
        self.btnSave.clicked.connect(self.save)
        self.btnExport.clicked.connect(lambda: self.export(self.get_data()))
        self.topSliders = [self.topRollSlider,
                           self.topPitchSlider,
                           self.topYawSlider]
        self.bottomSliders = [self.bottomRollSlider,
                              self.bottomPitchSlider,
                              self.bottomYawSlider]
        self.topSBs = [self.topRollSB,
                       self.topPitchSB,
                       self.topYawSB]
        self.bottomSBs = [self.bottomRollSB,
                          self.bottomPitchSB,
                          self.bottomYawSB]
        for i in range(3):
            self.topSliders[i].valueChanged.connect(self.setTopBySlider)
            self.bottomSliders[i].valueChanged.connect(self.setBottomBySlider)
            self.topSBs[i].valueChanged.connect(self.setTopBySpinBox)
            self.bottomSBs[i].valueChanged.connect(self.setBottomBySpinBox)
        self.update_signal.connect(self.update_data)

        if self.nao.is_connected():
            self.connect(self.nao)

    def export(self, data):
        if data is None:
            return
        location = qtw.QFileDialog.getSaveFileName(
            self, "Save file",
            os.getcwd() + "/../../etc/configuration/location/default/head/" +
            self.model["subscribe_key"].split(".")[-1] + ".json")

        if location[0] == '':
            return

        try:
            f = open(location[0], 'w')
            json.dump(data, f, indent=4)
            f.write("\n")
            f.close()
        except Exception as e:
            logger.error(__name__ +
                         ": Exception while saving config to file: " +
                         str(e))
            self.window().statusBar().showMessage(str(e))

    def get_data(self) -> typing.Dict:
        if self.data is None:
            problem = "No data available for export"
            logger.warning(__name__ + ": " + problem)
            self.window().statusBar().showMessage(problem)
            return None
        data = self.data
        data["top_ext"] = self.getTopFromSB()
        data["bottom_ext"] = self.getBottomFromSB()
        return data

    def floatToSlider(self, v):
        return int(v * 100000)

    def sliderFloat(self, v):
        return (float(v) * 0.00001)

    def update_data(self, data: nd.ConfigMount):
        self.data = data.data
        top = data.data["top_ext"]
        bottom = data.data["bottom_ext"]
        for i in range(3):
            self.topSliders[i].setValue(self.floatToSlider(top[i]))
            self.bottomSliders[i].setValue(self.floatToSlider(bottom[i]))

    def connect(self, nao: Nao):
        self.nao = nao
        if self.model["subscribe_key"]:
            self.subscribe(self.model["subscribe_key"], True)

    def set(self):
        self.setTopBySlider()
        self.setBottomBySlider()

    def getTopFromSliders(self):
        return [self.sliderFloat(self.topSliders[0].value()),
                self.sliderFloat(self.topSliders[1].value()),
                self.sliderFloat(self.topSliders[2].value())]

    def getBottomFromSliders(self):
        return [self.sliderFloat(self.bottomSliders[0].value()),
                self.sliderFloat(self.bottomSliders[1].value()),
                self.sliderFloat(self.bottomSliders[2].value())]

    def getTopFromSB(self):
        return [self.topSBs[0].value(),
                self.topSBs[1].value(),
                self.topSBs[2].value()]

    def getBottomFromSB(self):
        return [self.bottomSBs[0].value(),
                self.bottomSBs[1].value(),
                self.bottomSBs[2].value()]

    def setTopBySlider(self):
        top = self.getTopFromSliders()
        for i in range(3):
            self.topSBs[i].setValue(top[i])
        self.nao.config_protocol.set(self.model["subscribe_key"],
                                     "top_ext", top)

    def setBottomBySlider(self):
        bottom = self.getBottomFromSliders()
        for i in range(3):
            self.bottomSBs[i].setValue(bottom[i])
        self.nao.config_protocol.set(self.model["subscribe_key"],
                                     "bottom_ext", bottom)

    def setTopBySpinBox(self):
        top = self.getTopFromSB()
        for i in range(3):
            self.topSliders[i].setValue(self.floatToSlider(top[i]))
        self.nao.config_protocol.set(self.model["subscribe_key"],
                                     "top_ext", top)

    def setBottomBySpinBox(self):
        bottom = self.getBottomFromSB()
        for i in range(3):
            self.bottomSliders[i].setValue(self.floatToSlider(bottom[i]))
        self.nao.config_protocol.set(self.model["subscribe_key"],
                                     "bottom_ext", bottom)

    def save(self):
        if self.nao.is_connected():
            self.nao.config_protocol.save()
        else:
            problem = "Cannot save, not connected"
            logger.warning(__name__ + ": " + problem)
            self.window().statusBar().showMessage(problem)

    def subscribe(self, key: str, force=False):
        if self.nao.is_connected():
            if key != self.model["subscribe_key"] or force:
                self.nao.config_protocol.unsubscribe(
                    self.model["subscribe_key"],
                    self.identifier)
                self.nao.config_protocol.subscribe(
                    key,
                    self.identifier,
                    lambda d: self.update_signal.emit(d))
        self.model["subscribe_key"] = key

    def unsubscribe(self):
        if self.nao.is_connected():
            self.nao.config_protocol.unsubscribe(self.model["subscribe_key"],
                                                 self.identifier)

    def closeEvent(self, event):
        if self.nao.is_connected():
            self.unsubscribe()
            self.nao.config_protocol.unsubscribe_msg_type(
                net_utils.ConfigMsgType.send_mounts, self.identifier)
        self.deleteLater()
        super(Main, self).closeEvent(event)
