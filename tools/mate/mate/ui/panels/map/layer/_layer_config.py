from abc import abstractmethod
import PyQt5.QtCore as qtc
from mate.net.nao import Nao


class _LayerConfig(qtc.QObject):

    @abstractmethod
    def connect(self, nao: Nao):
        pass

    @abstractmethod
    def closeEvent(self, event):
        pass

    @abstractmethod
    def reset_widgets(self):
        pass

    @abstractmethod
    def accept(self):
        pass

    @abstractmethod
    def discard(self):
        pass
