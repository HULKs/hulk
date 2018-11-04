import uuid
import json
import os

import PyQt5.QtCore as qtc
import PyQt5.QtWidgets as qtw

import mate.net.nao as nao
import mate.net.nao_data as nd
import mate.net.utils as netutils
from .config_view import Ui_DockWidget


class Config(qtw.QDockWidget):
    updateHandler = qtc.pyqtSignal(nd.ConfigMount)

    def __init__(self,
                 nao: nao.Nao,
                 subscribe_key: str = netutils.NO_SUBSCRIBE_KEY):
        super(Config, self).__init__()
        self.nao = nao
        self.identifier = uuid.uuid4()
        self.currentSubscribe = subscribe_key
        self.data = None
        self.data_orig = None

        self.ui = Ui_DockWidget()
        self.ui.setupUi(self)

        self.ui.btnSet.clicked.connect(self.set)
        self.ui.btnSave.clicked.connect(self.save)
        self.ui.btnExport.clicked.connect(lambda: self.export(self.get_data()))
        self.ui.btnExportDiff.clicked.connect(
            lambda: self.export(self.get_data_changed()))

        self.ui.cbxMount.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbxMount.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.ui.cbxMount.activated[str].connect(self.select_mount)
        self.ui.cbxMount.setCurrentText(self.currentSubscribe)

        self.updateHandler.connect(self.update)

        if self.nao.is_connected():
            self.connect(self.nao)

    def export(self, data):
        location = qtw.QFileDialog.getSaveFileName(
            self, "Save file",
            os.getcwd() + "/" + self.currentSubscribe.split(".")[-1] + ".json")

        if location[0] == '':
            return

        try:
            f = open(location[0], 'w')
            json.dump(data, f, indent=4)
            f.close()
        except Exception as e:
            self.window().statusBar().showMessage(str(e))

    def get_data(self):
        data = {}
        for row in range(self.ui.tblConfig.rowCount()):
            data[self.ui.tblConfig.item(row, 0).text()] = json.loads(
                self.ui.tblConfig.item(row, 1).text())
        return data

    def get_data_changed(self):
        data = {}
        for row in range(self.ui.tblConfig.rowCount()):
            index = self.ui.tblConfig.item(row, 0).text()
            table_entry = json.loads(self.ui.tblConfig.item(row, 1).text())
            if table_entry != self.data_orig.data[index]:
                data[index] = table_entry
        return data

    def fill_drop_down(self):
        self.ui.cbxMount.clear()
        if self.currentSubscribe not in self.nao.config_data:
            self.ui.cbxMount.addItem(self.currentSubscribe)
        for key in self.nao.config_data.keys():
            self.ui.cbxMount.addItem(key)
        self.ui.cbxMount.setCurrentText(self.currentSubscribe)

    def select_mount(self, mount):
        if self.nao.is_connected():
            self.subscribe(mount)

    def update(self, data: nd.ConfigMount):
        self.data = data
        self.data_orig = data
        self.ui.tblConfig.setRowCount(len(self.data.data.keys()))
        self.ui.tblConfig.setColumnCount(2)
        for index, key in enumerate(self.data.data.keys()):
            value = self.data.data[key]
            self.ui.tblConfig.setItem(index, 0, qtw.QTableWidgetItem(key))
            self.ui.tblConfig.setItem(index, 1,
                                      qtw.QTableWidgetItem(json.dumps(value)))

    def connect(self, nao: nao.Nao):
        self.nao = nao

        self.fill_drop_down()
        self.nao.config_protocol.subscribe_msg_type(
            netutils.ConfigMsgType.send_mounts, self.identifier,
            self.fill_drop_down)

        if self.currentSubscribe != netutils.NO_SUBSCRIBE_KEY:
            self.subscribe(self.currentSubscribe)

    def set(self):
        for index in range(self.ui.tblConfig.rowCount()):
            key = self.ui.tblConfig.item(index, 0).text()
            value = json.loads(self.ui.tblConfig.item(index, 1).text())
            self.nao.config_protocol.set(self.currentSubscribe, key, value)

    def save(self):
        self.nao.config_protocol.save()

    def closeEvent(self, event):
        if self.nao.is_connected():
            self.unsubscribe()
            self.nao.config_protocol.unsubscribe_msg_type(
                netutils.ConfigMsgType.send_mounts, self.identifier)
        self.deleteLater()
        super(Config, self).closeEvent(event)

    def subscribe(self, mount: str):
        self.unsubscribe()
        self.nao.config_protocol.subscribe(
            mount, self.identifier, lambda d: self.updateHandler.emit(d))
        self.currentSubscribe = mount

    def unsubscribe(self):
        self.nao.config_protocol.unsubscribe(self.currentSubscribe,
                                             self.identifier)
        self.currentSubscribe = None
