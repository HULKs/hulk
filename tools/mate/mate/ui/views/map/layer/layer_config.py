from abc import ABC, abstractmethod, ABCMeta
import PyQt5.QtCore as qtc
import mate.net.nao as nao


class LayerConfigMeta(type(qtc.QObject), ABCMeta):
    pass


class LayerConfig(ABC):

    @abstractmethod
    def connect(self, nao: nao.Nao):
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
