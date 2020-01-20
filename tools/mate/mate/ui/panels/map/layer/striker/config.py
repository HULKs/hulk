import PyQt5.QtWidgets as qtw
import PyQt5.QtCore as qtc

import uuid
import os

from mate.ui.panels.map.layer._layer_config import _LayerConfig
from mate.net.nao import Nao
import mate.net.utils as net_utils
import mate.ui.utils as ui_utils


class Config(qtw.QWidget, _LayerConfig):
    def __init__(self, layer_model, parent, update_callback, nao: Nao):
        super(Config, self).__init__(parent)
        ui_utils.loadUi(__file__, self)

        self.nao = nao
        self.layer_model = ui_utils.load_model(os.path.dirname(__file__) +
                                               "/model.json", layer_model)
        self.update_callback = update_callback
        self.identifier = uuid.uuid4()

        self.config_to_ui = {
            "center_x": (
                lambda: self.spin_center_x.value(),
                lambda value: self.spin_center_x.setValue(value)),
            "center_y": (
                lambda: self.spin_center_y.value(),
                lambda value: self.spin_center_y.setValue(value)),
            "kickRatingChunksKey": (
                lambda: self.cbx_kickRatingChunksKey.currentText(),
                lambda value: self.cbx_kickRatingChunksKey.setCurrentText(value)),
            "kickRatingChunkWeightsKey": (
                lambda: self.cbx_kickRatingChunkWeightsKey.currentText(),
                lambda value: self.cbx_kickRatingChunkWeightsKey.setCurrentText(value)),
            "rateKickKey": (
                lambda: self.cbx_rateKick.currentText(),
                lambda value: self.cbx_rateKick.setCurrentText(value)),
            "hitPointsKey": (
                lambda: self.cbx_hitPointsKey.currentText(),
                lambda value: self.cbx_hitPointsKey.setCurrentText(value)),
            "teamBallPositionKey": (
                lambda: self.cbx_teamBallPositionKey.currentText(),
                lambda value: self.cbx_teamBallPositionKey.setCurrentText(value)),
            "firstShadowPointKey": (
                lambda: self.cbx_firstShadowPointKey.currentText(),
                lambda value: self.cbx_firstShadowPointKey.setCurrentText(value)),
            "secondShadowPointKey": (
                lambda: self.cbx_secondShadowPointKey.currentText(),
                lambda value: self.cbx_secondShadowPointKey.setCurrentText(value)),
            "firstShadowPointAfterKey": (
                lambda: self.cbx_firstShadowPointAfterKey.currentText(),
                lambda value: self.cbx_firstShadowPointAfterKey.setCurrentText(value)),
            "secondShadowPointAfterKey": (
                lambda: self.cbx_secondShadowPointAfterKey.currentText(),
                lambda value: self.cbx_secondShadowPointAfterKey.setCurrentText(value)),
            }
        self.cbx_kickRatingChunksKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbx_kickRatingChunkWeightsKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbx_rateKick.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbx_hitPointsKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbx_teamBallPositionKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbx_firstShadowPointKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbx_secondShadowPointKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbx_firstShadowPointAfterKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.cbx_secondShadowPointAfterKey.completer().setFilterMode(qtc.Qt.MatchContains)

        self.btnAccept.pressed.connect(self.accept)
        self.btnDiscard.pressed.connect(self.discard)
        self.reset_widgets()
        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: Nao):
        self.nao = nao
        self.fill_cbx()
        self.nao.debug_protocol.subscribe_msg_type(
            net_utils.DebugMsgType.list, self.identifier, self.fill_cbx)

    def closeEvent(self, event):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe_msg_type(
                net_utils.DebugMsgType.list, self.identifier)

    def reset_widgets(self):
        self.edit_name.setText(self.layer_model["name"])
        self.enabledCheckBox.setChecked(self.layer_model["enabled"])
        for key in self.config_to_ui:
            self.config_to_ui[key][1](self.layer_model["config"][key])

    def fill_cbx(self):
        ui_utils.init_cbx(
            self.cbx_kickRatingChunksKey,
            self.layer_model["config"]["kickRatingChunksKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.cbx_kickRatingChunkWeightsKey,
            self.layer_model["config"]["kickRatingChunkWeightsKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.cbx_rateKick,
            self.layer_model["config"]["rateKickKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.cbx_hitPointsKey,
            self.layer_model["config"]["hitPointsKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.cbx_teamBallPositionKey,
            self.layer_model["config"]["teamBallPositionKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.cbx_firstShadowPointKey,
            self.layer_model["config"]["firstShadowPointKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.cbx_secondShadowPointKey,
            self.layer_model["config"]["secondShadowPointKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.cbx_firstShadowPointAfterKey,
            self.layer_model["config"]["firstShadowPointAfterKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.cbx_secondShadowPointAfterKey,
            self.layer_model["config"]["secondShadowPointAfterKey"],
            self.nao.debug_data)

    def accept(self):
        self.layer_model["name"] = self.edit_name.text()
        self.layer_model["enabled"] = self.enabledCheckBox.isChecked()
        for key in self.config_to_ui:
            self.layer_model["config"][key] = self.config_to_ui[key][0]()
        self.update_callback(self.layer_model)

    def discard(self):
        self.reset_widgets()
