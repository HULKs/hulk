import sys
import uuid
import signal
import os

import PyQt5.QtCore as qtc
import PyQt5.QtWidgets as qtw

import mate.net.utils as netutils
import mate.ui.views as views
import mate.net.nao as nao
import mate.ui.views.map.model as mapmodel
import mate.ui.views.plot.model as plotmodel
from .main_view import Ui_MainWindow


class Main(qtc.QObject):
    connection_established_signal = qtc.pyqtSignal()

    def __init__(self, config: str):
        super(Main, self).__init__()
        self.nao = nao.Nao()
        self.identifier = uuid.uuid4()

        self.config_dir = config

        self.connection_established_signal.connect(self._on_connection_established)

        self.window = qtw.QMainWindow()
        self.ui = Ui_MainWindow()
        self.ui.setupUi(self.window)

        self.ui.actionExit.triggered.connect(self.window.close)
        self.ui.actionConfiguration.triggered.connect(
            lambda: self.newConfigView())
        self.ui.actionImageView.triggered.connect(lambda: self.newImageView())
        self.ui.actionTextView.triggered.connect(lambda: self.newTextView())
        self.ui.actionMapView.triggered.connect(lambda: self.newMapView())
        self.ui.actionPlotView.triggered.connect(lambda: self.newPlotView())
        self.ui.actionCameraCalib.triggered.connect(lambda: self.newCameraCalibView())
        self.ui.actionRequest_lists.triggered.connect(
            lambda: self.request_lists())
        self.ui.actionClose_all.triggered.connect(lambda: self.close_all_widgets())

        self.ui.cbxSelectNao.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbxSelectNao.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)
        self.ui.cbxSelectNao.lineEdit().returnPressed.connect(self.connect)

        self.ui.btnConnectNao.clicked.connect(self.connect)
        self.ui.btnDisconnectNao.clicked.connect(self.disconnect)

        self.ui.btnLoad.clicked.connect(self.load_layout)
        self.ui.btnSave.clicked.connect(self.save_layout)
        self.ui.actionSave.triggered.connect(self.save_layout)

        self.ui.toolBar.addWidget(self.ui.connect_to_label)
        self.ui.toolBar.addWidget(self.ui.cbxSelectNao)
        self.ui.toolBar.addWidget(self.ui.btnConnectNao)
        self.ui.toolBar.addWidget(self.ui.btnDisconnectNao)

        self.ui.toolBar_spacer = qtw.QWidget()
        self.ui.toolBar_spacer.setMinimumWidth(10)
        self.ui.toolBar.addWidget(self.ui.toolBar_spacer)

        self.ui.toolBar.addWidget(self.ui.layout_label)
        self.ui.toolBar.addWidget(self.ui.cbxSelectLayout)
        self.ui.toolBar.addWidget(self.ui.btnLoad)
        self.ui.toolBar.addWidget(self.ui.btnSave)

        self.ui.centralwidget.hide()

        self.settings = qtc.QSettings(self.config_dir + "main.config",
                                      qtc.QSettings.NativeFormat)
        self.fill_layout_cbx()
        self.restore()

    def restore(self):
        if self.settings.value("cbxSelectLayout"):
            self.ui.cbxSelectLayout.setCurrentText(
                self.settings.value("cbxSelectLayout"))
            self.load_layout()
        if self.settings.value("cbxSelectNao"):
            self.ui.cbxSelectNao.setCurrentText(
                self.settings.value("cbxSelectNao"))

    def fill_layout_cbx(self):
        selected_layout = self.ui.cbxSelectLayout.currentText()
        self.ui.cbxSelectLayout.clear()
        os.makedirs(self.config_dir, exist_ok=True)
        for f in sorted(os.listdir(self.config_dir), key=str.lower):
            split_ext = os.path.splitext(f)
            if split_ext[1] == ".config" and split_ext[0] != "main":
                self.ui.cbxSelectLayout.addItem(split_ext[0])
        self.ui.cbxSelectLayout.setCurrentIndex(
            self.ui.cbxSelectLayout.findText(selected_layout))

    def close_all_widgets(self):
        children = self.window.findChildren(qtw.QDockWidget)
        for child in children:
            child.close()

    def load_layout(self):
        if self.ui.cbxSelectLayout.currentText():
            layout_settings = qtc.QSettings(
                self.config_dir + self.ui.cbxSelectLayout.currentText() +
                ".config", qtc.QSettings.NativeFormat)

            if layout_settings.value("geometry"):
                if layout_settings.value("docked_widgets"):
                    for child in self.window.findChildren(qtw.QDockWidget):
                        child.setParent(None)
                        child.close()

                    for widget in layout_settings.value("docked_widgets"):
                        if widget[0] == "Image":
                            self.newImageView(widget[1], widget[2])
                        elif widget[0] == "Text":
                            self.newTextView(widget[1], widget[2])
                        elif widget[0] == "Config":
                            self.newConfigView(widget[1], widget[2])
                        elif widget[0] == "Map":
                            self.newMapView(widget[1], widget[2])
                        elif widget[0] == "Plot":
                            self.newPlotView(widget[1], widget[2])
                        elif widget[0] == "CameraCalib":
                            self.newCameraCalibView(widget[1])

                    if layout_settings.value("geometry"):
                        self.window.restoreGeometry(
                            layout_settings.value("geometry").data())
                    if layout_settings.value("state"):
                        self.window.restoreState(
                            layout_settings.value("state").data())
                    self.ui.statusbar.showMessage("Load Layout from {}".format(
                        self.ui.cbxSelectLayout.currentText()))
                else:
                    self.ui.statusbar.showMessage("Loaded empty Layout!")
            else:
                self.ui.statusbar.showMessage("Could not load layout file!")
        else:
            self.ui.statusbar.showMessage("Please specify a layout name!")

    def save_layout(self):
        if self.ui.cbxSelectLayout.currentText():
            layout_settings = qtc.QSettings(
                self.config_dir + self.ui.cbxSelectLayout.currentText() +
                ".config", qtc.QSettings.NativeFormat)
            layout_settings.clear()
            children = self.window.findChildren(qtw.QDockWidget)
            docked_widgets = []
            for child in children:
                if type(child) == views.Map:
                    docked_widgets.append([
                        type(child).__name__, child.map_model,
                        child.objectName()
                    ])
                elif type(child) == views.Plot:
                    docked_widgets.append([
                        type(child).__name__, child.model,
                        child.objectName()
                    ])
                elif type(child) == views.CameraCalib:
                    docked_widgets.append([type(child).__name__, child.objectName()])
                else:
                    docked_widgets.append([
                        type(child).__name__, child.currentSubscribe,
                        child.objectName()
                    ])

            layout_settings.setValue("docked_widgets", docked_widgets)

            layout_settings.setValue("geometry", self.window.saveGeometry())
            layout_settings.setValue("state", self.window.saveState())

            del layout_settings

            self.fill_layout_cbx()
            self.ui.statusbar.showMessage("Save layout to {}".format(
                self.ui.cbxSelectLayout.currentText()))
        else:
            self.ui.statusbar.showMessage("Please specify a layout name!")

    def newMapView(self,
                   map_model: mapmodel.MapModel = None,
                   object_name: str = None):
        if map_model is None:
            map_model = mapmodel.MapModel()
        if object_name is None:
            object_name = str(uuid.uuid4())
        view = views.Map(self.nao, map_model)
        view.setObjectName(object_name)
        self.window.addDockWidget(qtc.Qt.BottomDockWidgetArea, view)

    def newImageView(self,
                     subscribe_key: str = netutils.NO_SUBSCRIBE_KEY,
                     object_name: str = None):
        if object_name is None:
            object_name = str(uuid.uuid4())
        view = views.Image(self.nao, subscribe_key)
        view.setObjectName(object_name)
        self.window.addDockWidget(qtc.Qt.BottomDockWidgetArea, view)
        view.ui.cbxMount.setFocus()

    def newTextView(self,
                    subscribe_key: str = netutils.NO_SUBSCRIBE_KEY,
                    object_name: str = None):
        if object_name is None:
            object_name = str(uuid.uuid4())
        view = views.Text(self.nao, subscribe_key)
        view.setObjectName(object_name)
        self.window.addDockWidget(qtc.Qt.BottomDockWidgetArea, view)
        view.ui.cbxMount.setFocus()

    def newConfigView(self,
                      subscribe_key: str = netutils.NO_SUBSCRIBE_KEY,
                      object_name: str = None):
        if object_name is None:
            object_name = str(uuid.uuid4())
        view = views.Config(self.nao, subscribe_key)
        view.setObjectName(object_name)
        self.window.addDockWidget(qtc.Qt.BottomDockWidgetArea, view)
        view.ui.cbxMount.setFocus()

    def newPlotView(self,
                    model: plotmodel.PlotModel = None,
                    object_name: str = None):
        if model is None:
            model = plotmodel.PlotModel()
        if object_name is None:
            object_name = str(uuid.uuid4())
        view = views.Plot(self.nao, model)
        view.setObjectName(object_name)
        self.window.addDockWidget(qtc.Qt.BottomDockWidgetArea, view)

    def newCameraCalibView(self,
                           object_name: str = None):
        if object_name is None:
            object_name = str(uuid.uuid4())
        view = views.CameraCalib(self.nao)
        view.setObjectName(object_name)
        self.window.addDockWidget(qtc.Qt.BottomDockWidgetArea, view)

    def run(self):
        self.window.show()

    def exit(self):
        self.disconnect()

        if (self.nao.debug_thread.is_alive()):
            self.nao.debug_thread.join()
        if (self.nao.config_thread.is_alive()):
            self.nao.config_thread.join()

        self.settings.setValue("cbxSelectNao",
                               self.ui.cbxSelectNao.currentText())
        self.settings.setValue("cbxSelectLayout",
                               self.ui.cbxSelectLayout.currentText())
        del self.settings

    def connect(self):
        selected_nao = self.ui.cbxSelectNao.currentText()

        self.ui.statusbar.showMessage(
            "Trying to connect {}".format(selected_nao))
        # TODO: in nao.connect() an exception occurs when the connection could not be established. Has to be catched.
        self.nao.connect(selected_nao, post_hook=lambda: self.connection_established_signal.emit())

    def _on_connection_established(self):
        for child in self.window.findChildren(qtw.QDockWidget):
            child.connect(self.nao)

        self.ui_connect()

        self.ui.statusbar.showMessage("Connect to {}".format(self.nao.nao_address))

        self.nao.debug_protocol.subscribe_status(
            netutils.ConnectionStatusType.connection_lost, self.identifier,
            self.connection_lost)

    def connection_lost(self):
        self.ui.statusbar.showMessage("Connection to {} lost!".format(
            self.nao.nao_address))
        self.ui_disconnect()

    def disconnect(self):
        if self.nao.is_connected():
            self.nao.disconnect()
            self.ui.statusbar.showMessage("Disconnected.")
            self.ui_disconnect()

    def request_lists(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.send_debug_msg(
                netutils.DebugMsgType.request_list)
            self.nao.config_protocol.send_config_msg(
                netutils.ConfigMsgType.get_mounts)
            self.ui.statusbar.showMessage("Requested new lists")
        else:
            self.ui.statusbar.showMessage(
                "Cannot request lists: not connected")

    def ui_connect(self):
        self.ui.cbxSelectNao.setEnabled(False)
        self.ui.btnConnectNao.setEnabled(False)
        self.ui.btnDisconnectNao.setEnabled(True)
        self.ui.menuConnection.setEnabled(True)

    def ui_disconnect(self):
        self.ui.cbxSelectNao.setEnabled(True)
        self.ui.btnConnectNao.setEnabled(True)
        self.ui.btnDisconnectNao.setEnabled(False)
        self.ui.menuConnection.setEnabled(False)
