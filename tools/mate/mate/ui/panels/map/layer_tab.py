import time

import PyQt5.QtWidgets as qtw

import mate.ui.utils as ui_utils
from mate.net.nao import Nao
from . import util
from mate.debug.colorlog import ColorLog

logger = ColorLog()


class LayerTab(qtw.QWidget):
    def __init__(self, parent, nao: Nao):
        super(LayerTab, self).__init__(parent)
        ui_utils.loadUi(__file__, self)
        self.parent = parent
        self.nao = nao

        self.btnAddLayer.setMenu(qtw.QMenu(self.btnAddLayer))
        for name in self.parent.layer_modules:
            self.btnAddLayer.menu().addAction(
                name,
                lambda layer=name: self.add_layer(layer))

        self.btnDeleteLayer.clicked.connect(self.delete_selected_layer)
        self.btnMoveUp.clicked.connect(self.move_layer_up)
        self.btnMoveDown.clicked.connect(self.move_layer_down)

        self.listWidget.itemSelectionChanged.connect(
            self.layer_selected)

        self.configWidget.hide()

        self.update_list()

    def connect(self, nao):
        self.nao = nao
        if self.configWidget.isVisible():
            self.configWidget.connect(self.nao)

    def add_layer(self, layer_type: str):
        logger.debug(__name__ + ": Adding " + layer_type + " map layer")
        addTime = time.time()
        self.parent.model["layer"].append(util.create_layer(layer_type))
        self.layer_selected()
        self.update_list()
        logger.debug(__name__ +
                     ": Adding " + layer_type + " map layer took: " +
                     logger.timerLogStr(addTime))

    def update_list(self):
        self.listWidget.clear()
        for layer in self.parent.model["layer"]:
            self.listWidget.addItem(layer["name"])

    def delete_selected_layer(self):
        current_row = self.listWidget.currentRow()
        # ensure that the list of layers is not empty
        if self.parent.model["layer"]:
            self.parent.model["layer"].pop(current_row)
            if current_row > 0:
                current_row = current_row - 1
            self.listWidget.setCurrentRow(current_row)
            self.update_list()

    def move_layer_up(self):
        current_row = self.listWidget.currentRow()
        if current_row > 0:
            util.swap_layer(self.parent.model["layer"],
                            current_row, current_row - 1)
            self.update_list()

    def move_layer_down(self):
        current_row = self.listWidget.currentRow()
        if current_row < self.listWidget.count() - 1:
            util.swap_layer(self.parent.model["layer"],
                            current_row, current_row + 1)
            self.update_list()

    def layer_model_changed(self, new_model: dict):
        for layerIndex in range(len(self.parent.model["layer"])):
            if self.parent.model["layer"
                                 ][layerIndex
                                   ]["identifier"] == new_model["identifier"]:
                self.parent.model["layer"][layerIndex] = new_model
                break
        self.update_list()

    def layer_selected(self):
        self.configWidget.close()
        if self.listWidget.currentRow() < len(self.parent.model["layer"]):
            self.parent.model["selected_index"] = self.listWidget.currentRow()
            layer_model = self.parent.model[
                "layer"][self.parent.model["selected_index"]]
            self.configWidget = self.parent.layer_modules[
                layer_model["type"]].Config(layer_model,
                                            self.splitter,
                                            self.layer_model_changed,
                                            self.nao)
