import mate.net.utils as netutils
import uuid
from enum import Enum


class Curve():
    def __init__(self, name: str, enabled: bool, key: str, key_lambda: str, color: str):
        self.name = name
        self.identifier = uuid.uuid4()
        self.enabled = enabled
        self.key = key
        self.key_lambda = key_lambda
        self.color = color


class TabType(Enum):
    plot = 0
    config = 1


class PlotModel():
    def __init__(self):
        self.selected_tab = TabType(1)
        self.selected_curve = None
        self.curves = []
        self.fps = 30
        self.buffer_size = 10
        self.show_legend = True

    def select_tab(self, index: int):
        self.selected_tab = TabType(index)

    def select_curve(self, index: int):
        if index < len(self.curves):
            self.selected_curve = self.curves[index]
        else:
            self.selected_curve = None

    def add_curve(self,
                  name: str = "Curve",
                  enabled: bool = True,
                  key: str = netutils.NO_SUBSCRIBE_KEY,
                  key_lambda: str = "output = input",
                  color: str = "#000000"):
        self.curves.append(Curve(name, enabled, key, key_lambda, color))

    def delete_curve(self, index: int):
        self.curves.pop(index)
