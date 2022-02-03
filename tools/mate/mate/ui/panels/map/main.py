import importlib
import typing as ty

import PyQt5.QtWidgets as qtw
import PyQt5.QtGui as qtg
import os

from mate.ui.panels._panel import _Panel
from mate.ui.panels.map.layer_tab import LayerTab
from mate.ui.panels.map.config_tab import ConfigTab
from mate.ui.panels.map.map_tab import MapTab
from mate.net.nao import Nao
import mate.ui.utils as ui_utils
import mate.ui.panels.map.util as util
from mate.debug.colorlog import ColorLog

logger = ColorLog()


class Main(_Panel):
    name = "Map"
    shortcut = qtg.QKeySequence("Ctrl+M")

    def __init__(self, main_window, nao: Nao, model=None):
        super(Main, self).__init__(main_window, self.name, nao)
        ui_utils.loadUi(__file__, self)
        self.model = ui_utils.load_model(os.path.dirname(__file__) +
                                         "/model.json", model)
        self.layer_modules = self.import_layer()

        self.map_tab = MapTab(self, self.nao, self.model["projection_corners"])
        self.layer_tab = LayerTab(self, self.nao)
        self.config_tab = ConfigTab(self)

        self.tabWidget.addTab(self.map_tab, "Map")
        self.tabWidget.addTab(self.layer_tab, "Layer")
        self.tabWidget.addTab(self.config_tab, "Config")
        self.tabWidget.setCurrentIndex(self.model["selected_tab"])
        if self.is_map_tab_selected():
            self.map_tab.create_painter()

        self.tabWidget.currentChanged.connect(self.tab_changed)

    @staticmethod
    def import_layer() -> ty.Dict:
        layer = importlib.import_module('mate.ui.panels.map.layer')
        layer_modules = {}
        for possible in sorted(os.listdir(os.path.dirname(layer.__file__))):
            if possible[0] == "_":
                continue
            try:
                module = importlib.import_module("." + possible,
                                                 "mate.ui.panels.map.layer")
                logger.info(__name__ + ": Importing " +
                            possible + " map layer")
                layer_modules[module.Main.name] = module
            except Exception as e:
                logger.error(__name__ + ": Exception when importing " +
                             possible + " map layer:")
                logger.error(__name__ + ": " + str(e))
        return layer_modules

    def is_map_tab_selected(self):
        return self.model["selected_tab"] == util.TabType.MAP.value

    def connect(self, nao: Nao):
        self.nao = nao

        self.map_tab.connect(self.nao)
        self.layer_tab.connect(self.nao)

    def tab_changed(self, index: int):
        self.model["selected_tab"] = index

        if self.is_map_tab_selected():
            self.map_tab.create_painter()
        else:
            self.map_tab.destroy_painter()

    def closeEvent(self, event):
        self.map_tab.destroy_painter()
        self.deleteLater()
        super(Main, self).closeEvent(event)
