import os
import time
from collections import deque

import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg
import PyQt5.QtWidgets as qtw
import pyqtgraph as pygraph
import pyqtgraph.exporters as exporters

import mate.net.nao_data as nao_data
import mate.net.utils as net_utils
import mate.ui.utils as ui_utils
from mate.net.nao import Nao
from mate.ui.panels._panel import _Panel
from . import util
from mate.debug.colorlog import ColorLog

logger = ColorLog()

pygraph.setConfigOption('background', 'w')
pygraph.setConfigOption('foreground', 'k')


class Main(_Panel):
    name = "Plot"
    shortcut = qtg.QKeySequence("Ctrl+P")
    data_received_signal = qtc.pyqtSignal(nao_data.DebugValue, str)

    def __init__(self, main_window, nao: Nao, model=None):
        super(Main, self).__init__(main_window, self.name, nao)
        ui_utils.loadUi(__file__, self)
        self.model = ui_utils.load_model(os.path.dirname(__file__) +
                                         "/model.json", model)

        """
        self.data contains the data of curves.
        each curve data is a deque in this list:
            deque:
                - dict
                    - "y": value # this key should be called 'y'; pyqtgraph
                    - "timestamp": time received
            deque: ....
        """
        self.data = {}
        self.should_update = False
        self.plots = []

        self.cbxKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbxKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)

        self.data_received_signal.connect(self.data_received)
        self.spinFps.valueChanged.connect(self.set_fps)
        self.spinBufferSize.valueChanged.connect(self.set_buffer_size)
        self.legendCheckBox.stateChanged.connect(self.set_show_legend)

        self.listWidget.itemSelectionChanged.connect(self.select_curve)
        self.btnAccept.clicked.connect(self.accept)
        self.btnDiscard.clicked.connect(self.discard)
        self.btnAddCurve.clicked.connect(self.add_curve)
        self.btnDeleteCurve.clicked.connect(self.delete_curve)
        self.btnColor.clicked.connect(self.select_color)
        self.btnSnap.clicked.connect(self.snap)
        self.edit_color.returnPressed.connect(
            lambda: ui_utils.reset_textField_color(
                self.edit_color,
                self.edit_color.text()))

        self.update_list()

        self.timer = qtc.QTimer()
        self.timer.timeout.connect(self.update)

        self.reset_fps_spin()
        self.reset_buffer_size_spin()
        self.reset_show_legend()

        self.tabWidget.currentChanged.connect(self.tab_changed)
        self.tabWidget.setCurrentIndex(self.model["selected_tab"])

        self._init_datalist()
        self._init_plot()

        self.data_received_signal.connect(self.data_received)

        if self.nao.is_connected():
            self.connect(self.nao)

    def set_fps(self):
        self.model["fps"] = self.spinFps.value()

    def snap(self):
        # Get size of plotItem and create ImageExporter
        width = self.plot_widget.plotItem.size().width()
        height = self.plot_widget.plotItem.size().height()
        exporter = exporters.ImageExporter(self.plot_widget.plotItem)
        # Set resolution, force int type, new value may not be == to old value
        exporter.params.param('width'). \
            setValue(int(width * 4), blockSignal=exporter.widthChanged)
        exporter.params.param('height'). \
            setValue(int(height * 4), blockSignal=exporter.heightChanged)
        # Set filepath and export
        location_suggestion = os.path.join(os.getcwd(), "plot.png")
        location, _ = qtw.QFileDialog. \
            getSaveFileName(self.widget(),
                            "Save snap",
                            location_suggestion,
                            options=qtw.QFileDialog.Options())
        if location == '':
            # If export is cancelled, exit gracefully
            logger.debug(__name__ + ": Saving Snapshot aborted.")
            return
        exporter.export(location)

    def reset_fps_spin(self):
        self.spinFps.setValue(self.model["fps"])

    def set_buffer_size(self):
        self.model["buffer_size"] = self.spinBufferSize.value()

    def reset_buffer_size_spin(self):
        self.spinBufferSize.setValue(self.model["buffer_size"])

    def set_show_legend(self):
        self.model["show_legend"] = self.legendCheckBox.isChecked()

    def reset_show_legend(self):
        self.legendCheckBox.setChecked(self.model["show_legend"])

    def select_color(self):
        ui_utils.pick_color(self.edit_color, self.edit_color.text())

    def fill_drop_down(self):
        self.cbxKey.clear()
        for key, data in self.nao.debug_data.items():
            if not data.isImage:
                self.cbxKey.addItem(key)

    def connect(self, nao: Nao):
        self.nao = nao
        self.fill_drop_down()
        self.nao.debug_protocol.subscribe_msg_type(
            net_utils.DebugMsgType.list, self.identifier, self.fill_drop_down)
        self.subscribe_keys()
        self.timer.start(1000 / self.model["fps"])
        # self.init_plot()
        # self.init_datalist()

    def tab_changed(self, index: int):
        self.model["selected_tab"] = index

        if self.model["selected_tab"] == util.TabType.plot.value:
            self.subscribe_keys()
            self._init_datalist()
            self._init_plot()
            self.update()
            if self.nao.is_connected():
                self.timer.start(1000 / self.model["fps"])
        else:
            self.unsubscribe_keys()
            self.timer.stop()

    def _init_plot(self):
        self.plot_widget.clear()
        if self.plot_widget.plotItem.legend is not None:
            self.plot_widget.plotItem.legend.scene().removeItem(
                self.plot_widget.plotItem.legend)
            self.plot_widget.plotItem.legend = None
        if self.model["show_legend"]:
            self.plot_widget.addLegend()

        self.plots = []
        for curve in self.model["curves"]:
            if curve["enabled"]:
                color = curve["color"]
            else:
                color = "#555555"
            self.plots.append(
                self.plot_widget.plot(
                    name=curve["name"], pen={
                        'color': color,
                        'width': 2
                    }))

    def _init_datalist(self):
        self.data = {}
        for curve in self.model["curves"]:
            self.data[curve["identifier"]] = deque()

    def update_list(self):
        self.listWidget.clear()
        for curve in self.model["curves"]:
            self.listWidget.addItem(curve["name"])

    def update(self):
        if self.should_update:
            for curve, plot in zip(self.model["curves"], self.plots):
                plot.setData(self.data[curve["identifier"]])
            self.should_update = False

    def data_received(self, data: net_utils.Data, identifier: str):
        data = self.apply_lambda(data, identifier)
        while len(self.data[identifier]) > 0 and time.time() - self.data[identifier][0]["timestamp"] > self.model["buffer_size"]:
            self.data[identifier].popleft()
        if type(data) is list:
            for datum in data:
                self.data[identifier].append({"y": datum, "timestamp": time.time()})
        else:
            self.data[identifier].append({"y": data, "timestamp": time.time()})
        self.should_update = True

    def apply_lambda(self, data: net_utils.Data, identifier: str):
        scope = {"input": data.data, "output": None}
        filtered = list(filter(lambda curve: curve["identifier"] == identifier,
                               self.model["curves"]))
        if len(filtered):
            exec(filtered[0]["key_lambda"], scope)
        else:
            # TODO: Do something useful here
            logger.warning("Curve not found by identifier " + identifier)
        return scope["output"]

    def select_curve(self):
        util.select_curve(self.listWidget.currentRow(), self.model)
        if self.model["selected_curve"] is not None:
            self.reset_curve_config()
            self.formWidget.setEnabled(True)

    @property
    def selected_curve_config(self):
        return self.model["curves"][self.model["selected_curve"]]

    def reset_curve_config(self):
        self.nameLineEdit.setText(self.selected_curve_config["name"])
        self.enabledCheckBox.setChecked(self.selected_curve_config["enabled"])
        self.cbxKey.setCurrentText(self.selected_curve_config["key"])
        self.edit_lambda.setText(self.selected_curve_config["key_lambda"])
        ui_utils.reset_textField_color(self.edit_color,
                                       self.selected_curve_config["color"])

    def accept(self):
        self.model["curves"][self.model["selected_curve"]] = \
            util.create_curve(self.nameLineEdit.text(),
                              self.enabledCheckBox.isChecked(),
                              self.cbxKey.currentText(),
                              self.edit_lambda.toPlainText(),
                              self.edit_color.text())
        self.update_list()

    def discard(self):
        self.select_curve()

    def add_curve(self):
        self.model["curves"].append(util.create_curve())
        self.update_list()

    def delete_curve(self):
        # TODO: Delete does not update config section
        self.model["curves"].pop(self.listWidget.currentRow())
        self.update_list()
        if self.listWidget.count() == 0:
            self.formWidget.setEnabled(False)

    def subscribe_keys(self):
        for curve in self.model["curves"]:
            if curve["enabled"]:
                self.subscribe(curve["key"], curve["identifier"])

    def subscribe(self, key: str, identifier: str):
        if self.nao.is_connected():
            self.nao.debug_protocol.subscribe(
                key, identifier,
                lambda d: self.data_received_signal.emit(d, identifier))

    def unsubscribe_keys(self):
        if self.nao.is_connected():
            for curve in self.model["curves"]:
                if curve["enabled"]:
                    self.nao.debug_protocol.unsubscribe(curve["key"],
                                                        curve["identifier"])

    def closeEvent(self, event):
        self.unsubscribe_keys()
        self.deleteLater()
        super(Main, self).closeEvent(event)
