import PyQt5.QtGui as qtg
import PyQt5.QtCore as qtc
import PyQt5.QtWidgets as qtw
import os

import mate.net.utils as netutils
from mate.ui.views.view.view_controller import View


class Image(View):
    def __init__(self, nao, subscribe_key: str = netutils.NO_SUBSCRIBE_KEY):
        super(Image, self).__init__(nao, subscribe_key)
        self.setWindowTitle("Image")
        self.pixmap = qtg.QPixmap()

    def fill_drop_down(self):
        self.ui.cbxMount.clear()
        if self.currentSubscribe not in self.nao.debug_data:
            self.ui.cbxMount.addItem(self.currentSubscribe)
        for key, data in self.nao.debug_data.items():
            if data.isImage:
                self.ui.cbxMount.addItem(key)
        self.ui.cbxMount.setCurrentText(self.currentSubscribe)

    def update(self):
        if not self.should_update:
            return
        self.pixmap.loadFromData(self.data.data)

        w = self.ui.label.width()
        h = self.ui.label.height()

        self.ui.label.setMinimumSize(1, 1)
        self.ui.label.setPixmap(
            self.pixmap.scaled(w, h, qtc.Qt.KeepAspectRatio))

        self.should_update = False

    def snap(self):
        location_suggestion = os.path.join(os.getcwd(), "{}.png".format(
            self.currentSubscribe))
        location, _ = qtw.QFileDialog.getSaveFileName(self, "Save snap",
                                                      location_suggestion)
        if location == '':
            return
        self.pixmap.save(location)
