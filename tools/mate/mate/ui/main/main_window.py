import importlib
import json
import os
import typing
import uuid
import time
import logging

import PyQt5.QtCore as qtc
import PyQt5.QtWidgets as qtw

import mate.net.nao as nao
import mate.net.nao_data as nd
import mate.net.utils as net_utils
import mate.ui.utils as ui_utils
from mate.ui.panels._panel import _Panel
from mate.debug.colorlog import ColorLog

from hulks import aliveness

logger = ColorLog()


class MainWindow(qtw.QMainWindow):
    connection_established_signal = qtc.pyqtSignal()
    connection_lost_signal = qtc.pyqtSignal()
    connection_failure_signal = qtc.pyqtSignal(Exception)
    nao_info_received = qtc.pyqtSignal(nd.ConfigMount)

    def __init__(self,
                 layout_dir: str,
                 panel_dir: str,
                 verbose: bool,
                 timeout: float,
                 default_config_dir: str):
        logger.debug(__name__ + ": Setting up main window")
        init_time = time.time()
        super(MainWindow, self).__init__()
        ui_utils.loadUi(__file__, self)
        self.nao = nao.Nao()
        self.identifier = uuid.uuid4()

        self.default_config_dir = default_config_dir
        self.layout_dir = layout_dir
        self.panel_dir = panel_dir
        self.verbose = verbose
        self.panel_modules = {}
        self.import_panels()

        # Set Nao info when connected
        self.nao_info_received.connect(
            lambda mount: self.set_nao_head_body_number(mount))

        self.connection_established_signal.connect(
            self._on_connection_established)
        self.connection_lost_signal.connect(
            self._on_connection_lost)
        self.connection_failure_signal.connect(self._on_connection_failure)
        self.actionExit.triggered.connect(self.close)
        self.actionRequest_lists.triggered.connect(
            lambda: self.request_lists())
        self.actionClose_all.triggered.connect(
            lambda: self.close_all_panels())
        self.actionDebug.triggered.connect(
            lambda checked, level=logging.DEBUG:
                self.set_log_level(level))
        self.actionInfo.triggered.connect(
            lambda checked, level=logging.INFO:
                self.set_log_level(level))
        self.actionWarning.triggered.connect(
            lambda checked, level=logging.WARNING:
                self.set_log_level(level))
        self.actionError.triggered.connect(
            lambda checked, level=logging.ERROR:
                self.set_log_level(level))
        self.actionOff.triggered.connect(
            lambda checked, level=logging.CRITICAL:
                self.set_log_level(level))
        self.cbxSelectNao.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbxSelectNao.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.cbxSelectNao.lineEdit().returnPressed.connect(self.connect)

        self.btnConnectNao.clicked.connect(self.connect)
        self.btnDisconnectNao.clicked.connect(self.disconnect)

        self.btnLoad.clicked.connect(self.load_layout)
        self.btnSave.clicked.connect(self.save_layout)
        self.actionSave.triggered.connect(self.save_layout)

        self.toolBar.addWidget(self.connect_to_label)
        self.toolBar.addWidget(self.cbxSelectNao)
        self.toolBar.addWidget(self.btnConnectNao)
        self.toolBar.addWidget(self.btnDisconnectNao)

        self.toolBar_spacer = qtw.QWidget()
        self.toolBar_spacer.setMinimumWidth(10)
        self.toolBar.addWidget(self.toolBar_spacer)

        self.toolBar.addWidget(self.layout_label)
        self.toolBar.addWidget(self.cbxSelectLayout)
        self.toolBar.addWidget(self.btnLoad)
        self.toolBar.addWidget(self.btnSave)

        self.centralwidget.hide()

        self.settings = qtc.QSettings(self.layout_dir + "main.config",
                                      qtc.QSettings.NativeFormat)
        self.fill_layout_cbx()
        self.restore()

        if timeout is not None:
            self.nao.timeout = float(timeout)

        logger.debug(__name__ + ": Setting up main window took: " +
                     logger.timerLogStr(init_time))

    def set_log_level(self, level):
        logger.info(__name__ +
                    ": setting Log level to " +
                    logging.getLevelName(level))
        logger.setLevel(level)
        if level == logging.DEBUG:
            self.actionDebug.setChecked(True)
        else:
            self.actionDebug.setChecked(False)
        if level == logging.INFO:
            self.actionInfo.setChecked(True)
        else:
            self.actionInfo.setChecked(False)
        if level == logging.WARNING:
            self.actionWarning.setChecked(True)
        else:
            self.actionWarning.setChecked(False)
        if level == logging.ERROR:
            self.actionError.setChecked(True)
        else:
            self.actionError.setChecked(False)
        if level == logging.CRITICAL:
            self.actionOff.setChecked(True)
        else:
            self.actionOff.setChecked(False)

    def restore(self):
        if self.settings.value("cbxSelectLayout"):
            self.cbxSelectLayout.setCurrentText(
                self.settings.value("cbxSelectLayout"))
            self.load_layout()
        if self.settings.value("cbxSelectNao"):
            self.cbxSelectNao.setCurrentText(
                self.settings.value("cbxSelectNao"))
        if self.verbose:
            self.set_log_level(logging.DEBUG)
        elif self.settings.value("logLevel"):
            self.set_log_level(int(self.settings.value("logLevel")))
        if self.settings.value("timeout"):
            self.nao.timeout = float(self.settings.value("timeout"))

    def fill_layout_cbx(self):
        selected_layout = self.cbxSelectLayout.currentText()
        self.cbxSelectLayout.clear()
        os.makedirs(self.layout_dir, exist_ok=True)
        for f in sorted(os.listdir(self.layout_dir), key=str.lower):
            split_ext = os.path.splitext(f)
            if split_ext[1] == ".layout" and split_ext[0] != "main":
                self.cbxSelectLayout.addItem(split_ext[0])
        self.cbxSelectLayout.setCurrentIndex(
            self.cbxSelectLayout.findText(selected_layout))

    def import_panels(self):
        panels = importlib.import_module('mate.ui.panels')
        for possible in os.listdir(self.panel_dir):
            if possible[0] == "_":
                continue
            if not os.path.isfile(self.panel_dir +
                                  "/" +
                                  possible +
                                  "/main.py"):
                logger.warning(__name__ +
                               ": Skipping possible panel directory '" +
                               possible +
                               "' because it lacks a main.py")
                continue
            try:
                logger.info(__name__ + ": Importing " + possible + " panel")
                module = importlib.import_module("." + possible + ".main",
                                                 "mate.ui.panels")
                self.panel_modules[module.Main.name] = module
                create_panel = qtw.QAction("&" + module.Main.name, self)
                if hasattr(module.Main, "shortcut"):
                    try:
                        create_panel.setShortcut(module.Main.shortcut)
                    except TypeError as e:
                        logger.error(__name__ +
                                     ": Could not create shortcut for " +
                                     possible + " panel: " + e)
                create_panel.triggered.connect(
                    lambda x, name=str(module.Main.name): self.new_panel(name))
                self.menuNew.addAction(create_panel)
            except Exception as e:
                logger.error(__name__ + ": Exception when importing " +
                             possible + " panel:")
                logger.error(__name__ + ": " + str(e))

    def close_all_panels(self):
        logger.debug(__name__ + ": Closing panels")
        close_time = time.time()
        for child in self.findChildren(qtw.QDockWidget):
            child.close()
            child.setParent(None)
        logger.debug(__name__ +
                     ": Closing panels took: " +
                     logger.timerLogStr(close_time))

    def load_layout(self):
        try:
            if self.cbxSelectLayout.currentText():
                logger.debug(__name__ + ": Loading " +
                             self.cbxSelectLayout.currentText() +
                             " layout")
                load_time = time.time()
                layout_settings = qtc.QSettings(
                    self.layout_dir + self.cbxSelectLayout.currentText() +
                    ".layout", qtc.QSettings.NativeFormat)

                if "panels" in layout_settings.childGroups():
                    self.close_all_panels()

                    layout_settings.beginGroup("panels")
                    for entry in layout_settings.childGroups():
                        layout_settings.beginGroup(entry)
                        self.new_panel(
                            name=layout_settings.value("type"),
                            objectName=layout_settings.group().split('/')[1],
                            model=json.loads(layout_settings.value("model")))
                        layout_settings.endGroup()
                    layout_settings.endGroup()

                    self.statusbar.showMessage("Load Layout from {}".format(
                        self.cbxSelectLayout.currentText()))
                else:
                    self.statusbar.showMessage("Loaded empty Layout")
                    logger.warning(__name__ + ": Loaded empty Layout")
                if layout_settings.value("geometry"):
                    self.restoreGeometry(
                        layout_settings.value("geometry").data())
                if layout_settings.value("state"):
                    self.restoreState(
                        layout_settings.value("state").data())
                logger.debug(__name__ +
                             ": Loading " +
                             self.cbxSelectLayout.currentText() +
                             " layout took: " + logger.timerLogStr(load_time))
            else:
                self.statusbar.showMessage("Please specify a layout name")
        except json.decoder.JSONDecodeError as e:
            self.handle_layout_load_fail(e, "JSON corrupt?")
        except KeyError as e:
            self.handle_layout_load_fail(e, "Mount type missing?")
        except TypeError as e:
            self.handle_layout_load_fail(e, "JSON missing?")
        except Exception as e:
            self.handle_layout_load_fail(e, "Error Message: ")

    def handle_layout_load_fail(self, e: Exception, guess: str):
        # Print Log to console
        logger.error(__name__ + ": Loading layout " +
                     self.cbxSelectLayout.currentText() +
                     " failed with {}. ".format(type(e).__name__) + guess)
        logger.error("Message: " + str(e))
        # Notify user
        self.statusbar.showMessage("An error occurred. "
                                   "See console for details.")
        # Recover to default state
        self.actionClose_all.triggered.emit()

    def save_layout(self):
        if self.cbxSelectLayout.currentText():
            logger.debug(__name__ + ": Saving " +
                         self.cbxSelectLayout.currentText() +
                         " layout")
            save_time = time.time()
            layout_settings = qtc.QSettings(
                self.layout_dir + self.cbxSelectLayout.currentText() +
                ".layout", qtc.QSettings.NativeFormat)
            layout_settings.clear()
            children = self.findChildren(qtw.QDockWidget)

            layout_settings.beginGroup("panels")
            for child in children:
                if _Panel in child.__class__.__bases__:
                    layout_settings.beginGroup(child.objectName())
                    layout_settings.setValue("type", child.name)
                    layout_settings.setValue("model", json.dumps(child.model))
                    layout_settings.endGroup()
            layout_settings.endGroup()

            layout_settings.setValue("geometry", self.saveGeometry())
            layout_settings.setValue("state", self.saveState())

            del layout_settings

            self.fill_layout_cbx()
            self.statusbar.showMessage("Save layout to {}".format(
                self.cbxSelectLayout.currentText()))
            logger.debug(__name__ +
                         ": Saving " + self.cbxSelectLayout.currentText() +
                         " layout took: " + logger.timerLogStr(save_time))
        else:
            self.statusbar.showMessage("Please specify a layout name")

    def new_panel(self, name: str,
                  objectName: str = None,
                  model: typing.Dict = None):
        if model is None:
            panel: _Panel = self.panel_modules[name].Main(self, self.nao)
        else:
            panel: _Panel = self.panel_modules[name].Main(
                self, self.nao, model)
        if objectName is None:
            panel.setObjectName(str(uuid.uuid4()))
        else:
            panel.setObjectName(objectName)
        self.addDockWidget(qtc.Qt.BottomDockWidgetArea, panel)

    def exit(self):
        logger.info(__name__ + ": Quitting Mate gracefully")
        self.disconnect()

        if self.nao.debug_thread.is_alive():
            self.nao.debug_thread.join()
        if self.nao.config_thread.is_alive():
            self.nao.config_thread.join()

        self.settings.setValue("cbxSelectNao",
                               self.cbxSelectNao.currentText())
        self.settings.setValue("cbxSelectLayout",
                               self.cbxSelectLayout.currentText())
        self.settings.setValue("logLevel", logger.level)
        self.settings.setValue("timeout", self.nao.timeout)
        del self.settings

        logger.info(__name__ + ": The end.")

    def connect(self):
        self.ui_connect()
        selected_nao = self.cbxSelectNao.currentText()
        logger.info(__name__ + ": Connect to: " + selected_nao)

        self.statusbar.showMessage(
            "Trying to connect {}".format(selected_nao))

        self.nao.connect(selected_nao,
                         established_hook=lambda:
                             self.connection_established_signal.emit(),
                         failure_hook=lambda error:
                             self.connection_failure_signal.emit(error))

    def _on_connection_established(self):
        self.nao.debug_protocol.subscribe_msg_type(
                    net_utils.DebugMsgType.list, self.nao.identifier,
                    self.nao.debug_protocol.subscribe_queued)

        for child in self.findChildren(qtw.QDockWidget):
            child.connect(self.nao)

        self.statusbar.showMessage(
            "Connected to {}".format(self.nao.nao_address))

        self.nao.debug_protocol.subscribe_status(
            net_utils.ConnectionStatusType.connection_lost, self.identifier,
            lambda: self.connection_lost_signal.emit())
        logger.info(__name__ + ": " + self.nao.nao_address + " connected")

        # Get nao Head and body number
        self.nao.nao_head = None
        self.nao.nao_body = None
        self.nao.location = None
        self.nao.config_protocol.subscribe(
            "tuhhSDK.base",
            self.identifier,
            lambda mount: self.nao_info_received.emit(mount)
        )

    def _on_connection_failure(self, error):
        self.statusbar.showMessage("Connection failed: {}".format(error))
        logger.error(__name__ + ": Connection failed: {}".format(error))
        self.ui_disconnect()

    def _on_connection_lost(self):
        self.statusbar.showMessage("Connection to {} lost".format(
            self.nao.nao_address))
        logger.error(__name__ +
                     ": Connection to " + self.nao.nao_address + " lost")
        self.ui_disconnect()

    def disconnect(self):
        if self.nao.is_connected():
            self.nao.disconnect()
            self.statusbar.showMessage("Disconnected.")
            logger.info(__name__ + ": Disconnected")
            self.ui_disconnect()

    def request_lists(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.send_debug_msg(
                net_utils.DebugMsgType.request_list)
            self.nao.config_protocol.send_config_msg(
                net_utils.ConfigMsgType.get_mounts)
            self.statusbar.showMessage("Requested new lists")
            logger.info(__name__ + ": Requested new lists")
        else:
            self.statusbar.showMessage(
                "Cannot request lists: not connected")
            logger.error(__name__ + ": Cannot request lists: not connected")

    def ui_connect(self):
        self.cbxSelectNao.setEnabled(False)
        self.btnConnectNao.setEnabled(False)
        self.btnDisconnectNao.setEnabled(True)
        self.menuConnection.setEnabled(True)

    def ui_disconnect(self):
        self.cbxSelectNao.setEnabled(True)
        self.btnConnectNao.setEnabled(True)
        self.btnDisconnectNao.setEnabled(False)
        self.menuConnection.setEnabled(False)

    def set_nao_head_body_number(self, data: nd.ConfigMount):
        nao_info = data.data["RobotInfo"]
        self.nao.nao_head = nao_info["headName"]
        self.nao.nao_body = nao_info["bodyName"]
        self.nao.location = data.data["location"]
        self.nao.config_protocol.unsubscribe(data.key, self.objectName())
