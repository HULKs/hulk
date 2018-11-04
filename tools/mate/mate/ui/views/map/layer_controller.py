import PyQt5.QtWidgets as qtw

from mate.ui.views.map.model import MapModel
from mate.ui.views.map.model import LayerType
from mate.ui.views.map.layer_view import Ui_Layer

import mate.net.nao as nao


class LayerController(qtw.QWidget):
    def __init__(self, map_model: MapModel, nao: nao.Nao):
        super(LayerController, self).__init__()

        self.nao = nao
        self.map_model = map_model

        self.ui = Ui_Layer()
        self.ui.setupUi(self)

        self.ui.btnAddLayer.setMenu(qtw.QMenu(self.ui.btnAddLayer))
        for layer_type in LayerType:
            self.ui.btnAddLayer.menu().addAction(
                layer_type,
                lambda layer_type=layer_type: self.add_layer(layer_type))

        self.ui.btnDeleteLayer.clicked.connect(self.delete_selected_layer)
        self.ui.btnMoveUp.clicked.connect(self.move_layer_up)
        self.ui.btnMoveDown.clicked.connect(self.move_layer_down)

        self.ui.listWidget.itemSelectionChanged.connect(
            self.layer_selected)

        self.ui.widget.hide()

        self.update_list()

    def connect(self, nao: nao.Nao):
        self.nao = nao
        if self.ui.widget.isVisible():
            self.ui.widget.connect(self.nao)

    def add_layer(self, layer_type: str):
        self.map_model.add_layer(layer_type)

        self.layer_selected()

        self.update_list()

    def update_list(self):
        self.ui.listWidget.clear()

        for layer in self.map_model.layer:
            self.ui.listWidget.addItem(layer["name"])

    def delete_selected_layer(self):
        currentRow = self.ui.listWidget.currentRow()
        self.map_model.layer.pop(currentRow)
        if currentRow > 0:
            currentRow = currentRow - 1
        self.ui.listWidget.setCurrentRow(currentRow)
        self.update_list()

    def move_layer_up(self):
        currentRow = self.ui.listWidget.currentRow()
        if currentRow > 0:
            self.map_model.swap_layer(currentRow, currentRow - 1)
            self.update_list()

    def move_layer_down(self):
        currentRow = self.ui.listWidget.currentRow()
        if currentRow < self.ui.listWidget.count() - 1:
            self.map_model.swap_layer(currentRow, currentRow + 1)
            self.update_list()

    def layer_selected(self):
        self.map_model.select_layer(self.ui.listWidget.currentRow())
        self.ui.widget.close()
        if self.map_model.get_selected_layer() is not None:
            layer_type = self.map_model.get_selected_layer()["type"]
            self.ui.widget = LayerType[layer_type][0](
                self.map_model.get_selected_layer(),
                self.ui.splitter,
                self.update_list,
                self.nao)
