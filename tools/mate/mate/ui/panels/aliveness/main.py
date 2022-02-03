import typing
from datetime import datetime
import asyncio
import threading

import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg
import PyQt5.QtWidgets as qtw

import mate.ui.utils as ui_utils
from mate.debug.colorlog import ColorLog
from mate.net.nao import Nao
from mate.ui.panels._panel import _Panel
from hulks.aliveness import getAliveRobots

logger = ColorLog()


class Main(_Panel):
    name = "Aliveness"
    shortcut = qtg.QKeySequence("Ctrl+A")
    update_signal = qtc.pyqtSignal()

    def __init__(self, main_window, nao: Nao, model: typing.Dict = None):
        super(Main, self).__init__(main_window, "Aliveness", Nao())
        ui_utils.loadUi(__file__, self)

        self.update_signal.connect(self.update_gui)
        self.query_aliveness()

        self.refresh_button.clicked.connect(self.query_aliveness)

        self.robots = []

    def query_aliveness(self):
        self.window().statusbar.showMessage("Querying aliveness...", 2000)
        self.refresh_button.setEnabled(False)
        self.aliveness_table.setEnabled(False)
        # Fetch robot data
        threading.Thread(target=self.aliveness_update_thread,
                         daemon=True).start()

    def aliveness_update_thread(self) -> None:
        self.robots = asyncio.run(getAliveRobots())
        self.update_signal.emit()

    def update_gui(self):
        self.aliveness_table.setSortingEnabled(False)
        self.aliveness_table.setRowCount(0)
        # Row count is the number of robots
        self.aliveness_table.setRowCount(len(self.robots))

        # Set good text color (copied from Config panel)
        palette = self.aliveness_table.palette()
        text_color = qtg.QColor(
            ui_utils.ideal_text_color(palette.base().color()))
        palette.setColor(qtg.QPalette.Text, text_color)

        # Go through robots and build the table
        for robot_index, (_, robot) in enumerate(self.robots.items()):
            # Compose list with relevant information
            self.aliveness_table.setItem(
                robot_index, 0, qtw.QTableWidgetItem(str(robot.head_number())))
            self.aliveness_table.setItem(
                robot_index, 1, qtw.QTableWidgetItem(str(robot.body_number())))
            self.aliveness_table.setItem(
                robot_index, 2, qtw.QTableWidgetItem(str(robot.player_number)))
            self.aliveness_table.setItem(
                robot_index, 3, qtw.QTableWidgetItem(str(robot.team_number)))
            self.aliveness_table.setItem(
                robot_index, 4, qtw.QTableWidgetItem(str(robot.eth_ip)))
            self.aliveness_table.setItem(
                robot_index, 5, qtw.QTableWidgetItem(str(robot.wifi_ip)))
            self.aliveness_table.setItem(
                robot_index, 6,
                qtw.QTableWidgetItem(
                    f"{int((datetime.now() - robot.timestamp).total_seconds() * 1000)} ms"
                ))

            # Add buttons to connect
            button_eth = qtw.QPushButton(
                robot.eth_ip if robot.eth_ip is not None else "N/A")
            button_eth.setEnabled(robot.eth_ip is not None)
            button_eth.clicked.connect(
                lambda _, i=robot.eth_ip: self.connect_nao(i))
            self.aliveness_table.setCellWidget(robot_index, 4, button_eth)

            button_wifi = qtw.QPushButton(
                robot.wifi_ip if robot.wifi_ip is not None else "N/A")
            button_wifi.setEnabled(robot.wifi_ip is not None)
            button_wifi.clicked.connect(
                lambda _, i=robot.wifi_ip: self.connect_nao(i))
            self.aliveness_table.setCellWidget(robot_index, 5, button_wifi)
        self.refresh_button.setEnabled(True)
        self.aliveness_table.resizeRowsToContents()
        self.aliveness_table.resizeColumnsToContents()
        self.aliveness_table.setSortingEnabled(True)
        self.aliveness_table.setEnabled(True)

    def connect_nao(self, ip: str):
        """Connect to nao in aliveness_table"""
        self.parent().disconnect()
        self.parent().cbxSelectNao.setItemText(1, ip)
        self.parent().cbxSelectNao.setCurrentIndex(1)
        self.parent().connect()

    def connect(self, _):
        # Do nothing, mandatory for connect from main window
        pass
