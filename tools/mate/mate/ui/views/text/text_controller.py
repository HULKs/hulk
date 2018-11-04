import json
import mate.net.utils as netutils
from mate.ui.views.view.view_controller import View

import PyQt5.QtGui as qtg


class Text(View):
    def __init__(self, nao, subscribe_key: str = netutils.NO_SUBSCRIBE_KEY):
        super(Text, self).__init__(nao, subscribe_key)
        self.setWindowTitle("Text")

    def fill_drop_down(self):
        self.ui.cbxMount.clear()
        if self.currentSubscribe not in self.nao.debug_data:
            self.ui.cbxMount.addItem(self.currentSubscribe)
        for key, data in self.nao.debug_data.items():
            if not data.isImage:
                self.ui.cbxMount.addItem(key)
        self.ui.cbxMount.setCurrentText(self.currentSubscribe)

    def update(self):
        if self.should_update:
            self.ui.label.setText(json.dumps(self.data.data, indent=2))
            self.should_update = False

    def snap(self):
        cb = qtg.QApplication.clipboard()
        cb.clear(mode=cb.Clipboard)
        cb.setText(self.ui.label.text(), mode=cb.Clipboard)
