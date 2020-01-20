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


def is_checkbox(tblConfig: qtw.QTableWidget, row: int) -> bool:
    """Returns true if QTableWidgetItem is a checkbox"""
    return tblConfig.item(row, 1).flags() == (qtc.Qt.ItemIsUserCheckable |
                                              qtc.Qt.ItemIsEnabled)


def is_checked(tblConfig: qtw.QTableWidget, row: int) -> bool:
    """Returns true if checkbox is checked"""
    return tblConfig.item(row, 1).checkState() == qtc.Qt.Checked


def get_key_value_from_tblConfig(tblConfig: qtw.QTableWidget, row: int) \
        -> (str, object):
    """Returns key and value from a given row of the given tblConfig"""
    key = tblConfig.item(row, 0).text()
    if is_checkbox(tblConfig, row):
        if is_checked(tblConfig, row):
            value = True
        else:
            value = False
    else:
        value = json.loads(tblConfig.item(row, 1).text())
    return key, value


class Main(_Panel):
    name = "Config"
    shortcut = qtg.QKeySequence("Ctrl+C")

    update_signal = qtc.pyqtSignal(nd.ConfigMount)

    def __init__(self, main_window, nao: Nao, model: typing.Dict = None):
        super(Main, self).__init__(main_window, self.name, nao)
        ui_utils.loadUi(__file__, self)
        self.model = ui_utils.load_model(os.path.dirname(__file__) +
                                         "/model.json", model)

        self.data = None
        self.data_orig = None

        # btnSetAll passes -1 to update all keys
        self.btnSetAll.clicked.connect(lambda: self.set(-1))
        self.btnSave.clicked.connect(self.save)
        self.btnExport.clicked.connect(lambda: self.export(self.get_data()))
        self.diffModeSelector.activated.connect(lambda index: self.export_diff(index))

        # Set values when pressing enter without klicking set-button
        self.tblConfig.cellChanged.connect(lambda row, col: self.set(row))

        self.cbxMount.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbxMount.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.cbxMount.activated[str].connect(self.select_mount)
        self.cbxMount.setCurrentText(self.model["subscribe_key"])

        self.update_signal.connect(self.update_data)

        if self.nao.is_connected():
            self.connect(self.nao)

    def export_diff(self, index: int):
        # Get default values
        if self.nao.is_connected():
            container = ui_utils.ConfigDiffInfoContainer(
                self.parent().default_config_dir, self.data.filename, self.nao, index)
        else:
            logger.info(__name__ + ": You don't seem to be connected to anything. Moving on.")
            return
        self.data_orig = ui_utils.get_default_config(container)

        # Get a suggestion where to put it
        suggestion = container.paths[container.mode]
        # If suggested directory does not exist, create a new directory
        if not os.path.isdir(os.path.dirname(suggestion)):
            os.makedirs(os.path.dirname(suggestion))
        # Get location and save to file
        location = ui_utils.get_file_location(
            panel=self.widget(), caption="Export Diff", suggestion=suggestion)
        if location == "":
            # Delete empty dir and empty parent dirs on abort
            try:
                os.removedirs(os.path.dirname(suggestion))
            except OSError:
                pass
            return
        if not ui_utils.save_dict_to_file(location, self.get_data_changed()):
            self.window().statusBar().showMessage("There has been an error saving the file.")
        # If directory is still empty, remove it
        try:
            os.removedirs(os.path.dirname(suggestion))
        except OSError:
            pass

    def export(self, data):
        location = ui_utils.get_file_location(
            panel=self.widget(), caption="Save file",
            suggestion=os.getcwd() + "/" + self.model["subscribe_key"].split(".")[-1] + ".json")
        if location == "":
            return
        if not ui_utils.save_dict_to_file(location, data):
            self.window().statusBar().showMessage("There has been an error saving the file.")

    def get_data(self) -> typing.Dict:
        data = {}
        for row in range(self.tblConfig.rowCount()):
            key, value = get_key_value_from_tblConfig(self.tblConfig, row)
            data[key] = value
        return data

    def get_data_changed(self):
        data = {}
        for row in range(self.tblConfig.rowCount()):
            key, value = get_key_value_from_tblConfig(self.tblConfig, row)
            # Try to get default value, if non-existent, use anyway
            try:
                if value != self.data_orig[key]:
                    data[key] = value
            except KeyError:
                data[key] = value
        return data

    def fill_drop_down(self):
        self.cbxMount.clear()
        if self.model["subscribe_key"] not in self.nao.config_data:
            self.cbxMount.addItem(self.model["subscribe_key"])
        for key in self.nao.config_data.keys():
            self.cbxMount.addItem(key)
        self.cbxMount.setCurrentText(self.model["subscribe_key"])

    def select_mount(self, mount):
        self.subscribe(mount)

    def update_data(self, data: nd.ConfigMount):
        # Block all Signals while updating
        self.tblConfig.blockSignals(True)
        self.data = data

        self.tblConfig.setRowCount(len(self.data.data.keys()))
        self.tblConfig.setColumnCount(2)
        palette = self.tblConfig.palette()
        text_color = qtg.QColor(ui_utils.ideal_text_color(
            palette.base().color()))
        palette.setColor(qtg.QPalette.Text, text_color)
        for index, key in enumerate(self.data.data.keys()):
            # Disable all functionality for keys
            dummy_key = qtw.QTableWidgetItem(key)
            dummy_key.setFlags(qtc.Qt.NoItemFlags)
            self.tblConfig.setItem(index, 0, dummy_key)

            value = self.data.data[key]
            # If value is a boolean, create a checkbox for it
            if isinstance(value, bool):
                dummy_value = qtw.QTableWidgetItem()
                if value:
                    dummy_value.setCheckState(qtc.Qt.Checked)
                else:
                    dummy_value.setCheckState(qtc.Qt.Unchecked)
                dummy_value.setFlags(qtc.Qt.NoItemFlags |
                                     qtc.Qt.ItemIsUserCheckable |
                                     qtc.Qt.ItemIsEnabled)
                self.tblConfig.setItem(index, 1, dummy_value)
            else:
                self.tblConfig.setItem(index, 1,
                                       qtw.QTableWidgetItem(json.dumps(value)))
        self.tblConfig.setPalette(palette)
        # Re-enable Signals
        self.tblConfig.blockSignals(False)

    def connect(self, nao: Nao):
        self.nao = nao

        self.fill_drop_down()
        self.nao.config_protocol.subscribe_msg_type(
            net_utils.ConfigMsgType.send_mounts, self.identifier,
            self.fill_drop_down)

        if self.model["subscribe_key"]:
            self.subscribe(self.model["subscribe_key"], True)

    def set(self, row: int):
        # param row: -1 to update all keys,
        # >= 0 to specify row of key in self.tblConfig to set
        if row >= 0:
            key, value = get_key_value_from_tblConfig(self.tblConfig, row)
            self.nao.config_protocol.set(self.model["subscribe_key"],
                                         key, value)
        else:
            for index in range(self.tblConfig.rowCount()):
                key, value = get_key_value_from_tblConfig(self.tblConfig, index)
                self.nao.config_protocol.set(self.model["subscribe_key"],
                                             key, value)

    def save(self):
        if self.nao.is_connected():
            self.nao.config_protocol.save()
        else:
            logger.info(__name__ + ": You don't seem to be connected to anything. Moving on.")

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
