import time
import uuid

import PyQt5.QtWidgets as qtw
from mate.net.nao import Nao
from mate.debug.colorlog import ColorLog

logger = ColorLog()


class _Panel(qtw.QDockWidget):
    def __init__(self, main_window: qtw.QMainWindow, name: str, nao: Nao):
        logger.debug(__name__ + ": Initializing " + name + " panel")
        initTime = time.time()
        super(_Panel, self).__init__(main_window)
        self.name = name
        self.identifier = str(uuid.uuid4())
        self.setWindowTitle(name)
        self.nao = nao
        self.model = None
        logger.debug(__name__ + ": Initializing " + name + " panel took: " +
                     logger.timerLogStr(initTime))
