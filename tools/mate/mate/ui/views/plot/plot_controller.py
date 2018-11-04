import time
import os
import uuid
from collections import deque

import PyQt5.QtWidgets as qtw
import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg
import pyqtgraph as pg
import pyqtgraph.exporters as exporters

import mate.net.nao as nao
import mate.net.nao_data as nao_data
import mate.net.utils as net_utils
import mate.ui.utils as ui_utils
from mate.ui.views.plot.model import PlotModel, TabType
from .plot_view import Ui_PlotView


class Plot(qtw.QDockWidget):
    updateSignal = qtc.pyqtSignal(nao_data.DebugValue, int)

    def __init__(self, nao: nao.Nao, model: PlotModel):
        super(Plot, self).__init__()

        self.nao = nao
        self.identifier = uuid.uuid4()
        self.model = model
        """
        self.data contains the data of curves.
        each curve data is a deque in this list:
            deque:
                - dict
                    - "y": value # this key should be called 'y'; pyqtgraph
                    - "timestamp": time received
            deque: ....
        """
        self.data = []
        self.should_update = False

        pg.setConfigOption('background', 'w')
        pg.setConfigOption('foreground', 'k')
        self.ui = Ui_PlotView()
        self.ui.setupUi(self)

        self.ui.cbxKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbxKey.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)

        self.ui.spin_fps.valueChanged.connect(self.set_fps)
        self.ui.spin_buffer.valueChanged.connect(self.set_buffer_size)
        self.ui.legendCheckBox.stateChanged.connect(self.set_show_legend)

        self.ui.listWidget.itemSelectionChanged.connect(self.select_curve)
        self.ui.btnAccept.clicked.connect(self.accept)
        self.ui.btnDiscard.clicked.connect(self.discard)
        self.ui.btnAddCurve.clicked.connect(self.add_curve)
        self.ui.btnDeleteCurve.clicked.connect(self.delete_curve)
        self.ui.btnColor.clicked.connect(self.select_color)
        self.ui.btnSnap.clicked.connect(self.snap)
        self.ui.edit_color.returnPressed.connect(
            lambda: ui_utils.reset_textField_color(
                self.ui.edit_color,
                self.ui.edit_color.text()))

        self.update_list()

        self.timer = qtc.QTimer()
        self.timer.timeout.connect(self.update)

        self.reset_fps_spin()
        self.reset_buffer_size_spin()
        self.reset_show_legend()

        self.ui.tabWidget.currentChanged.connect(self.tab_changed)
        self.ui.tabWidget.setCurrentIndex(self.model.selected_tab.value)

        self.updateSignal.connect(self.data_received)

        if self.nao.is_connected():
            self.connect(self.nao)

    def set_fps(self):
        self.model.fps = self.ui.spin_fps.value()

    def snap(self):
        exporter = exporters.ImageExporter(self.ui.plot_widget.plotItem)
        location_suggestion = os.path.join(os.getcwd(), "plot.png")
        location, _ = qtw.QFileDialog.getSaveFileName(self, "Save snap",
                                                      location_suggestion)
        exporter.export(location)

    def reset_fps_spin(self):
        self.ui.spin_fps.setValue(self.model.fps)

    def set_buffer_size(self):
        self.model.buffer_size = self.ui.spin_buffer.value()

    def reset_buffer_size_spin(self):
        self.ui.spin_buffer.setValue(self.model.buffer_size)

    def set_show_legend(self):
        self.model.show_legend = self.ui.legendCheckBox.isChecked()

    def reset_show_legend(self):
        self.ui.legendCheckBox.setChecked(self.model.show_legend)

    def select_color(self):
        ui_utils.pick_color(self.ui.edit_color, self.ui.edit_color.text())

    def fill_drop_down(self):
        self.ui.cbxKey.clear()
        for key, data in self.nao.debug_data.items():
            if not data.isImage:
                self.ui.cbxKey.addItem(key)

    def connect(self, nao: nao.Nao):
        self.nao = nao
        self.fill_drop_down()
        self.nao.debug_protocol.subscribe_msg_type(
            net_utils.DebugMsgType.list, self.identifier, self.fill_drop_down)
        self.subscribe_keys()
        self.init_plot()
        self.init_datalist()

    def tab_changed(self, index: int):
        self.model.select_tab(index)

        if self.model.selected_tab == TabType.plot:
            self.subscribe_keys()
            self.init_plot()
            self.init_datalist()
            self.timer.start(1000 / self.model.fps)
        else:
            self.unsubscribe_keys()
            self.timer.stop()

    def init_plot(self):
        self.ui.plot_widget.clear()
        # TODO: Create checkbox to disable legend
        if self.ui.plot_widget.plotItem.legend is not None:
            self.ui.plot_widget.plotItem.legend.scene().removeItem(
                self.ui.plot_widget.plotItem.legend)
            self.ui.plot_widget.plotItem.legend = None
        if self.model.show_legend:
            self.ui.plot_widget.addLegend()

        self.plots = []
        for curve in self.model.curves:
            if (curve.enabled):
                color = curve.color
            else:
                color = "#555555"
            self.plots.append(
                self.ui.plot_widget.plot(
                    name=curve.name, pen={
                        'color': color,
                        'width': 2
                    }))

    def init_datalist(self):
        self.data = []
        for curve in self.model.curves:
            self.data.append(deque())

    def update_list(self):
        self.ui.listWidget.clear()
        for curve in self.model.curves:
            self.ui.listWidget.addItem(curve.name)

    def update(self):
        if self.should_update:
            for i, plot in enumerate(self.plots):
                plot.setData(self.data[i])
            self.should_update = False

    def data_received(self, data: net_utils.Data, curve_index: int):
        data = self.apply_lambda(data, curve_index)
        self.data[curve_index].append({"y": data, "timestamp": time.time()})
        age_first_element = time.time() - self.data[curve_index][0]["timestamp"]
        if age_first_element > self.model.buffer_size:
            self.data[curve_index].popleft()
        self.should_update = True

    def apply_lambda(self, data: net_utils.Data, curve_index: int):
        scope = {"input": data.data, "output": None}
        exec(self.model.curves[curve_index].key_lambda, scope)
        return scope["output"]

    def select_curve(self):
        self.model.select_curve(self.ui.listWidget.currentRow())
        if self.model.selected_curve is not None:
            self.reset_curve_config()
            self.ui.formWidget.setEnabled(True)

    def reset_curve_config(self):
        self.ui.nameLineEdit.setText(self.model.selected_curve.name)
        self.ui.enabledCheckBox.setChecked(self.model.selected_curve.enabled)
        self.ui.cbxKey.setCurrentText(self.model.selected_curve.key)
        self.ui.edit_lambda.setText(self.model.selected_curve.key_lambda)
        ui_utils.reset_textField_color(self.ui.edit_color,
                                       self.model.selected_curve.color)

    def accept(self):
        self.model.selected_curve.__init__(self.ui.nameLineEdit.text(),
                                           self.ui.enabledCheckBox.isChecked(),
                                           self.ui.cbxKey.currentText(),
                                           self.ui.edit_lambda.toPlainText(),
                                           self.ui.edit_color.text())
        self.update_list()

    def discard(self):
        self.select_curve()

    def add_curve(self):
        self.model.add_curve()
        self.update_list()

    def delete_curve(self):
        # TODO: Delete does not update config section
        self.model.delete_curve(self.ui.listWidget.currentRow())
        self.update_list()
        if self.ui.listWidget.count() == 0:
            self.ui.formWidget.setEnabled(False)

    def subscribe_keys(self):
        for curve in self.model.curves:
            if (curve.enabled):
                self.subscribe(curve.key, self.model.curves.index(curve),
                               curve.identifier)

    def subscribe(self, key: str, curve_index: int, identifier):
        if self.nao.is_connected():
            self.nao.debug_protocol.subscribe(
                key, identifier,
                lambda d: self.updateSignal.emit(d, curve_index))

    def unsubscribe_keys(self):
        if self.nao.is_connected():
            for curve in self.model.curves:
                if (curve.enabled):
                    self.unsubscribe(curve.key, curve.identifier)

    def unsubscribe(self, key, identifier):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(key, identifier)

    def closeEvent(self, event):
        self.unsubscribe_keys()
        self.deleteLater()
        super(Plot, self).closeEvent(event)
