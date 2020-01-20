import typing
from datetime import datetime

import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg
import PyQt5.QtWidgets as qtw

import mate.ui.utils as ui_utils
from mate.debug.colorlog import ColorLog
from mate.lib.aliveness.loader import Loader as AlivenessLoader
from mate.net.nao import Nao
from mate.ui.panels._panel import _Panel

logger = ColorLog()


class Main(_Panel):
    name = "Aliveness"
    shortcut = qtg.QKeySequence("Ctrl+A")
    update_signal = qtc.pyqtSignal()
    loader = AlivenessLoader().start()

    def __init__(self, main_window, nao: Nao, model: typing.Dict = None):
        super(Main, self).__init__(main_window, "Aliveness", Nao())
        ui_utils.loadUi(__file__, self)

        self.loader.set_signal(self.update_signal)
        self.update_signal.connect(self.update_gui)

        self.robots = []

    def update_gui(self):
        # Fetch robot data
        self.robots = self.loader.robots()

        # Row count is the number of robots
        self.aliveness_table.setRowCount(len(self.robots))

        # Set good text color (copied from Config panel)
        palette = self.aliveness_table.palette()
        text_color = qtg.QColor(ui_utils.ideal_text_color(
            palette.base().color()))
        palette.setColor(qtg.QPalette.Text, text_color)

        # Go through robots and build the table
        for robot_index, robot in enumerate(self.robots):
            if robot.is_alive:
                hulk_service = "Yes"
            else:
                if robot.is_lan and robot.is_wlan:
                    hulk_service = "No"
                else:
                    hulk_service = "Unknown"

            # Compose list with relevant information
            row_contents = [
                robot.info.head_num,
                robot.info.body_num,
                robot.info.player_num,
                robot.last_address,
                "-%.2fs" % (datetime.now() - robot.timestamp).total_seconds(),
                hulk_service,
            ]

            # Column corresponds to index in row_contents
            for col, elem in enumerate(row_contents):
                self.aliveness_table.setItem(robot_index, col,
                                             qtw.QTableWidgetItem(str(elem)))

            # Set background color with respect to aliveness
            if robot.is_alive:
                self.aliveness_table.item(robot_index, 3) \
                    .setBackground(qtg.QColor(0, 255, 0))
            else:
                self.aliveness_table.item(robot_index, 3) \
                    .setBackground(qtg.QColor(255, 0, 0))

            # Add button to connect
            button = qtw.QPushButton("Connect")
            button.clicked.connect(
                lambda _, i=robot_index: self.connect_nao(i))
            self.aliveness_table.setCellWidget(
                robot_index, len(row_contents), button)

    def connect_nao(self, index):
        """Connect to nao depending on row in aliveness_table"""
        ip = self.robots[index].last_address
        self.parent().disconnect()
        self.parent().cbxSelectNao.setItemText(1, ip)
        self.parent().cbxSelectNao.setCurrentIndex(1)
        self.parent().connect()

    def connect(self, _):
        # Do nothing, mandatory for connect from main window
        pass
