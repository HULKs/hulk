import typing as ty

from mate.ui.panels.map.map_painter import Painter
from mate.net.nao import Nao


class _Layer:

    def __init__(self, layer_model: ty.Dict, nao: Nao, identifier: str):
        self.layer_model = layer_model
        self.config = self.layer_model["config"]
        self.nao = nao
        self.identifier = identifier

    def connect(self, nao: Nao):
        raise NotImplementedError("{} method needs to be defined by sub-class (layer)".format(self.connect.__name__))
        pass

    def destroy(self):
        raise NotImplementedError("{} method needs to be defined by sub-class (layer)".format(self.destroy.__name__))
        pass

    def paint(self, painter: Painter):
        raise NotImplementedError("{} method needs to be defined by sub-class (layer)".format(self.paint.__name__))
        pass
