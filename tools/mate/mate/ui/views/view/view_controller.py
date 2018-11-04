import uuid

import PyQt5.QtCore as qtc
import PyQt5.QtWidgets as qtw

import mate.net.nao as nao
import mate.net.utils as netutils

from .view_view import Ui_DebugView


class View(qtw.QDockWidget):
    def __init__(self,
                 nao: nao.Nao,
                 subscribe_key: str = netutils.NO_SUBSCRIBE_KEY):
        super(View, self).__init__()

        self.nao = nao
        self.identifier = uuid.uuid4()
        self.currentSubscribe = subscribe_key

        self.ui = Ui_DebugView()
        self.ui.setupUi(self)

        self.should_update = False
        self.data = None

        self.ui.cbxMount.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbxMount.completer().setCompletionMode(
            qtw.QCompleter.PopupCompletion)

        self.ui.cbxMount.activated[str].connect(self.subscribe)
        self.ui.cbxMount.setCurrentText(self.currentSubscribe)

        self.timer = qtc.QTimer()
        self.timer.timeout.connect(self.update)
        self.ui.spnFramerate.valueChanged.connect(self.set_timer)

        self.ui.btnSnap.clicked.connect(self.snap)

        if self.nao.is_connected():
            self.connect(self.nao)

    def set_timer(self, frameRate: int):
        self.timer.stop()
        if frameRate > 0 and self.nao.is_connected():
            self.timer.start(1000 / (frameRate))

    def connect(self, nao: nao.Nao):
        self.nao = nao
        self.set_timer(self.ui.spnFramerate.value())

        self.fill_drop_down()
        self.nao.debug_protocol.subscribe_msg_type(
            netutils.DebugMsgType.list, self.identifier, self.fill_drop_down)

        if self.currentSubscribe != netutils.NO_SUBSCRIBE_KEY:
            self.subscribe(self.currentSubscribe)

    def fill_drop_down(self):
        ...

    def snap(self):
        ...

    def subscribe(self, key):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(self.currentSubscribe,
                                                self.identifier)
            self.nao.debug_protocol.subscribe(key, self.identifier,
                                              lambda d: self.data_received(d))
            self.currentSubscribe = key

    def data_received(self, data: netutils.Data):
        self.data = data
        self.should_update = True

    def update(self):
        ...

    def closeEvent(self, event):
        if self.nao.is_connected():
            self.unsubscribe()
            self.nao.debug_protocol.unsubscribe_msg_type(
                netutils.DebugMsgType.list, self.identifier)
        self.timer.stop()
        self.deleteLater()
        super(View, self).closeEvent(event)

    def unsubscribe(self):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(self.currentSubscribe,
                                                self.identifier)
