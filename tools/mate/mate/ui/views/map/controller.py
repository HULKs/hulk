import PyQt5.QtWidgets as qtw
import uuid

from mate.ui.views.map.view import Ui_DockWidget
from mate.ui.views.map.map_view import MapView
from mate.ui.views.map.model import MapModel
from mate.ui.views.map.model import TabType
from mate.ui.views.map.layer_controller import LayerController
from mate.ui.views.map.config_controller import Config
import mate.net.nao as nao


class Map(qtw.QDockWidget):
    def __init__(self, nao, map_model: MapModel):
        super(Map, self).__init__()

        self.nao = nao
        self.identifier = uuid.uuid4()

        self.map_model = map_model

        self.setWindowTitle("MapView")

        self.ui = Ui_DockWidget()
        self.ui.setupUi(self)

        self.map_view = MapView(self.map_model, self.nao)
        self.layer_view = LayerController(self.map_model, self.nao)
        self.config_view = Config(self.map_model)

        self.ui.tabWidget.addTab(self.map_view, "Map")
        self.ui.tabWidget.addTab(self.layer_view, "Layer")
        self.ui.tabWidget.addTab(self.config_view, "Config")
        self.ui.tabWidget.setCurrentIndex(self.map_model.selected_tab.value)

        self.ui.tabWidget.currentChanged.connect(self.tab_changed)

    def connect(self, nao: nao.Nao):
        self.nao = nao

        self.map_view.connect(self.nao)
        self.layer_view.connect(self.nao)

    def tab_changed(self, index: int):
        self.map_model.select_tab(index)

        if self.map_model.selected_tab == TabType.map:
            self.map_view.create_painter()
        else:
            self.map_view.destroy_painter()

    def closeEvent(self, event):
        self.map_view.destroy_painter()
        self.deleteLater()
        super(Map, self).closeEvent(event)
