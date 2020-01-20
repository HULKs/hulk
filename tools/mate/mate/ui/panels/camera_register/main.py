import os
from enum import Enum

import PyQt5.QtCore as qtc

import mate.net.nao_data as nd
import mate.ui.utils as ui_utils
from mate.debug.colorlog import ColorLog
from mate.net.nao import Nao
from mate.ui.panels._panel import _Panel

logger = ColorLog()


class CameraType(Enum):
    TOP = 0
    BOTTOM = 1


class Main(_Panel):
    name = "Camera Register"

    signal_update_data = qtc.pyqtSignal(nd.ConfigMount)

    def __init__(self, main_window, nao: Nao, model=None):
        super(Main, self).__init__(main_window, self.name, nao)
        ui_utils.loadUi(__file__, self)
        self.model = ui_utils.load_model(os.path.dirname(__file__) +
                                         "/model.json", model)

        self.signal_update_data.connect(self.update_data)

        self.edit_top_value.textChanged.connect(lambda: self.update_value(CameraType.TOP, self.edit_top_value.text()))
        self.chk_top_bit0.stateChanged.connect(lambda: self.update_value(CameraType.TOP))
        self.chk_top_bit1.stateChanged.connect(lambda: self.update_value(CameraType.TOP))
        self.chk_top_bit2.stateChanged.connect(lambda: self.update_value(CameraType.TOP))
        self.chk_top_bit3.stateChanged.connect(lambda: self.update_value(CameraType.TOP))
        self.chk_top_bit4.stateChanged.connect(lambda: self.update_value(CameraType.TOP))
        self.chk_top_bit5.stateChanged.connect(lambda: self.update_value(CameraType.TOP))
        self.chk_top_bit6.stateChanged.connect(lambda: self.update_value(CameraType.TOP))
        self.chk_top_bit7.stateChanged.connect(lambda: self.update_value(CameraType.TOP))

        self.edit_bottom_value.textChanged.connect(
            lambda: self.update_value(CameraType.BOTTOM, self.edit_bottom_value.text()))
        self.chk_bottom_bit0.stateChanged.connect(lambda: self.update_value(CameraType.BOTTOM))
        self.chk_bottom_bit1.stateChanged.connect(lambda: self.update_value(CameraType.BOTTOM))
        self.chk_bottom_bit2.stateChanged.connect(lambda: self.update_value(CameraType.BOTTOM))
        self.chk_bottom_bit3.stateChanged.connect(lambda: self.update_value(CameraType.BOTTOM))
        self.chk_bottom_bit4.stateChanged.connect(lambda: self.update_value(CameraType.BOTTOM))
        self.chk_bottom_bit5.stateChanged.connect(lambda: self.update_value(CameraType.BOTTOM))
        self.chk_bottom_bit6.stateChanged.connect(lambda: self.update_value(CameraType.BOTTOM))
        self.chk_bottom_bit7.stateChanged.connect(lambda: self.update_value(CameraType.BOTTOM))

        self.edit_register_addr.returnPressed.connect(self.address_entered)

        # Buttons
        self.btn_set_top.clicked.connect(lambda: self.clicked_set(CameraType.TOP))
        self.btn_set_bottom.clicked.connect(lambda: self.clicked_set(CameraType.BOTTOM))

        if self.nao.is_connected():
            self.connect(self.nao)

    def update_value(self, camera: CameraType, text=None):
        if text is None:
            value = 0
            if camera == CameraType.TOP:
                if self.chk_top_bit0.isChecked(): value += 1
                if self.chk_top_bit1.isChecked(): value += 2
                if self.chk_top_bit2.isChecked(): value += 4
                if self.chk_top_bit3.isChecked(): value += 8
                if self.chk_top_bit4.isChecked(): value += 16
                if self.chk_top_bit5.isChecked(): value += 32
                if self.chk_top_bit6.isChecked(): value += 64
                if self.chk_top_bit7.isChecked(): value += 128
                self.edit_top_value.setText(str(value))
            else:
                if self.chk_bottom_bit0.isChecked(): value += 1
                if self.chk_bottom_bit1.isChecked(): value += 2
                if self.chk_bottom_bit2.isChecked(): value += 4
                if self.chk_bottom_bit3.isChecked(): value += 8
                if self.chk_bottom_bit4.isChecked(): value += 16
                if self.chk_bottom_bit5.isChecked(): value += 32
                if self.chk_bottom_bit6.isChecked(): value += 64
                if self.chk_bottom_bit7.isChecked(): value += 128
                self.edit_bottom_value.setText(str(value))
        else:
            if camera == CameraType.TOP:
                try:
                    value = int(text, 0)
                    self.btn_set_top.setEnabled(True)
                except:
                    self.btn_set_top.setEnabled(False)
                    return
                self.chk_top_bit0.setChecked(True if value & 1 else False)
                self.chk_top_bit1.setChecked(True if value & 2 else False)
                self.chk_top_bit2.setChecked(True if value & 4 else False)
                self.chk_top_bit3.setChecked(True if value & 8 else False)
                self.chk_top_bit4.setChecked(True if value & 16 else False)
                self.chk_top_bit5.setChecked(True if value & 32 else False)
                self.chk_top_bit6.setChecked(True if value & 64 else False)
                self.chk_top_bit7.setChecked(True if value & 128 else False)
            else:
                try:
                    value = int(text, 0)
                    self.btn_set_bottom.setEnabled(True)
                except:
                    self.btn_set_bottom.setEnabled(False)
                    return
                self.chk_bottom_bit0.setChecked(True if value & 1 else False)
                self.chk_bottom_bit1.setChecked(True if value & 2 else False)
                self.chk_bottom_bit2.setChecked(True if value & 4 else False)
                self.chk_bottom_bit3.setChecked(True if value & 8 else False)
                self.chk_bottom_bit4.setChecked(True if value & 16 else False)
                self.chk_bottom_bit5.setChecked(True if value & 32 else False)
                self.chk_bottom_bit6.setChecked(True if value & 64 else False)
                self.chk_bottom_bit7.setChecked(True if value & 128 else False)

    def update_data(self, d: nd.ConfigMount):
        if self.model["mount_topCamera"] in d.key:
            self.edit_register_addr.setText(str(hex(d.data.get("registerAddr"))))
            self.update_value(CameraType.TOP, str(d.data.get("registerValue")))
            self.set_enabled_top_edits(True)
        elif self.model["mount_bottomCamera"] in d.key:
            self.update_value(CameraType.BOTTOM, str(d.data.get("registerValue")))
            self.set_enabled_bottom_edits(True)
        else:
            logger.error(__name__ + ": Received invalid config key of mount " + d.key)

    def clicked_set(self, camera: CameraType):
        addr = int(self.edit_register_addr.text(), 0)

        if camera == CameraType.TOP:
            value = int(self.edit_top_value.text(), 0)
        else:
            value = int(self.edit_bottom_value.text(), 0)

        self.set_register_value(camera, addr, value)

    def set_register_value(self, camera: CameraType, addr: int, value: int):
        logger.debug(__name__ + ": set register value of addr {} to value {}".format(addr, value))
        if camera == CameraType.TOP:
            camera_mount = self.model["mount_topCamera"]
        else:
            camera_mount = self.model["mount_bottomCamera"]

        self.nao.config_protocol.set(camera_mount, "registerAddr", addr)
        self.nao.config_protocol.set(camera_mount, "registerValue", value)
        self.nao.config_protocol.set(camera_mount, "registerWrite", True)

    def address_entered(self):
        if self.nao.is_connected():
            self.set_enabled_top_edits(False)
            self.set_enabled_bottom_edits(False)
            addr = int(self.edit_register_addr.text(), 0)
            self.nao.config_protocol.set(self.model["mount_topCamera"], "registerAddr", addr)
            self.nao.config_protocol.set(self.model["mount_bottomCamera"], "registerAddr", addr)
            self.nao.config_protocol.set(self.model["mount_topCamera"], "registerWrite", False)
            self.nao.config_protocol.set(self.model["mount_bottomCamera"], "registerWrite", False)
            self.nao.config_protocol.request_keys(self.model["mount_topCamera"])
            self.nao.config_protocol.request_keys(self.model["mount_bottomCamera"])
            logger.debug(__name__ + ": Address entered. Keys for top and bottom requested")

    def connect(self, nao: Nao):
        self.nao = nao
        self.subscribe_camera_configs()

    def subscribe_camera_configs(self):
        self.nao.config_protocol.subscribe(
            self.model["mount_topCamera"],
            self.identifier,
            lambda d: self.signal_update_data.emit(d))
        self.nao.config_protocol.subscribe(
            self.model["mount_bottomCamera"],
            self.identifier,
            lambda d: self.signal_update_data.emit(d))

    def set_enabled_top_edits(self, enable: bool):
        self.edit_top_value.setEnabled(enable)
        self.chk_top_bit0.setEnabled(enable)
        self.chk_top_bit1.setEnabled(enable)
        self.chk_top_bit2.setEnabled(enable)
        self.chk_top_bit3.setEnabled(enable)
        self.chk_top_bit4.setEnabled(enable)
        self.chk_top_bit5.setEnabled(enable)
        self.chk_top_bit6.setEnabled(enable)
        self.chk_top_bit7.setEnabled(enable)
        self.btn_set_top.setEnabled(enable)

    def set_enabled_bottom_edits(self, enable: bool):
        self.edit_bottom_value.setEnabled(enable)
        self.chk_bottom_bit0.setEnabled(enable)
        self.chk_bottom_bit1.setEnabled(enable)
        self.chk_bottom_bit2.setEnabled(enable)
        self.chk_bottom_bit3.setEnabled(enable)
        self.chk_bottom_bit4.setEnabled(enable)
        self.chk_bottom_bit5.setEnabled(enable)
        self.chk_bottom_bit6.setEnabled(enable)
        self.chk_bottom_bit7.setEnabled(enable)
        self.btn_set_bottom.setEnabled(enable)

    def closeEvent(self, event):
        if self.nao.is_connected():
            self.nao.config_protocol.unsubscribe(
                self.model["mount_topCamera"],
                self.identifier)
            self.nao.config_protocol.unsubscribe(
                self.model["mount_bottomCamera"],
                self.identifier)
        self.deleteLater()
        super(Main, self).closeEvent(event)
