import PyQt5.QtWidgets as qtw


class App(qtw.QApplication):
    def __init__(self, argv):
        super(App, self).__init__(argv)
        self.setApplicationName("MATE")
        self.setApplicationVersion("v1.0")
        self.setOrganizationName("HULKs")

