from abc import ABC, abstractmethod

from mate.ui.views.map.map_painter import Painter
import mate.net.nao as nao


class Layer(ABC):

    @abstractmethod
    def connect(self, nao: nao.Nao):
        pass

    @abstractmethod
    def destroy(self):
        pass

    @abstractmethod
    def paint(self, painter: Painter):
        pass
