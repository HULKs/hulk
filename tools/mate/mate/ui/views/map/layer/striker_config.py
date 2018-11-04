import PyQt5.QtWidgets as qtw
import PyQt5.QtCore as qtc
import PyQt5.QtGui as qtg

import uuid

from mate.ui.views.map.layer.striker_config_view import Ui_StrikerConfig
from mate.ui.views.map.layer.layer_config import LayerConfig, LayerConfigMeta
import mate.net.nao as nao
import mate.net.utils as net_utils
import mate.ui.utils as ui_utils


class StrikerConfig(qtw.QWidget, LayerConfig, metaclass=LayerConfigMeta):
    def __init__(self, layer, parent, update_callback, nao: nao.Nao):
        super(StrikerConfig, self).__init__(parent)
        self.nao = nao
        self.layer = layer
        self.update_callback = update_callback
        self.identifier = uuid.uuid4()
        self.ui = Ui_StrikerConfig()
        self.ui.setupUi(self)
        if self.layer["settings"] is None:
            self.layer["settings"] = {
                "center_x": 5.2,
                "center_y": -3.7,
                "kickRatingChunksKey": "Brain.StrikerActionProvider.kickRatingChunks",
                "kickRatingChunkWeightsKey": "Brain.StrikerActionProvider.kickRatingChunkWeights",
                "rateKickKey": "Brain.StrikerActionProvider.rateKick",
                "hitPointsKey": "Brain.StrikerActionProvider.hitPoints",
                "teamBallPositionKey": "Brain.TeamBallFilter.teamBallModel",
                "firstShadowPointKey": "Brain.StrikerActionProvider.firstShadowPoint",
                "secondShadowPointKey": "Brain.StrikerActionProvider.secondShadowPoint",
                "firstShadowPointAfterKey": "Brain.StrikerActionProvider.firstShadowPointAfter",
                "secondShadowPointAfterKey": "Brain.StrikerActionProvider.secondShadowPointAfter"
            }
        self.settings_to_ui = {
            "center_x": (
                lambda: self.ui.spin_center_x.value(),
                lambda value: self.ui.spin_center_x.setValue(value)),
            "center_y": (
                lambda: self.ui.spin_center_y.value(),
                lambda value: self.ui.spin_center_y.setValue(value)),
            "kickRatingChunksKey": (
                lambda: self.ui.cbx_kickRatingChunksKey.currentText(),
                lambda value: self.ui.cbx_kickRatingChunksKey.setCurrentText(value)),
            "kickRatingChunkWeightsKey": (
                lambda: self.ui.cbx_kickRatingChunkWeightsKey.currentText(),
                lambda value: self.ui.cbx_kickRatingChunkWeightsKey.setCurrentText(value)),
            "rateKickKey": (
                lambda: self.ui.cbx_rateKick.currentText(),
                lambda value: self.ui.cbx_rateKick.setCurrentText(value)),
            "hitPointsKey": (
                lambda: self.ui.cbx_hitPointsKey.currentText(),
                lambda value: self.ui.cbx_hitPointsKey.setCurrentText(value)),
            "teamBallPositionKey": (
                lambda: self.ui.cbx_teamBallPositionKey.currentText(),
                lambda value: self.ui.cbx_teamBallPositionKey.setCurrentText(value)),
            "firstShadowPointKey": (
                lambda: self.ui.cbx_firstShadowPointKey.currentText(),
                lambda value: self.ui.cbx_firstShadowPointKey.setCurrentText(value)),
            "secondShadowPointKey": (
                lambda: self.ui.cbx_secondShadowPointKey.currentText(),
                lambda value: self.ui.cbx_secondShadowPointKey.setCurrentText(value)),
            "firstShadowPointAfterKey": (
                lambda: self.ui.cbx_firstShadowPointAfterKey.currentText(),
                lambda value: self.ui.cbx_firstShadowPointAfterKey.setCurrentText(value)),
            "secondShadowPointAfterKey": (
                lambda: self.ui.cbx_secondShadowPointAfterKey.currentText(),
                lambda value: self.ui.cbx_secondShadowPointAfterKey.setCurrentText(value)),
            }
        self.ui.cbx_kickRatingChunksKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbx_kickRatingChunkWeightsKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbx_rateKick.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbx_hitPointsKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbx_teamBallPositionKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbx_firstShadowPointKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbx_secondShadowPointKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbx_firstShadowPointAfterKey.completer().setFilterMode(qtc.Qt.MatchContains)
        self.ui.cbx_secondShadowPointAfterKey.completer().setFilterMode(qtc.Qt.MatchContains)

        self.ui.btnAccept.pressed.connect(self.accept)
        self.ui.btnDiscard.pressed.connect(self.discard)
        self.reset_widgets()
        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: nao.Nao):
        self.nao = nao
        self.fill_cbx()
        self.nao.debug_protocol.subscribe_msg_type(
            net_utils.DebugMsgType.list, self.identifier, self.fill_cbx)

    def closeEvent(self, event):
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe_msg_type(
                net_utils.DebugMsgType.list, self.identifier)

    def reset_widgets(self):
        self.ui.edit_name.setText(self.layer["name"])
        self.ui.enabledCheckBox.setChecked(self.layer["enabled"])
        for key in self.settings_to_ui:
            self.settings_to_ui[key][1](self.layer["settings"][key])

    def fill_cbx(self):
        ui_utils.init_cbx(
            self.ui.cbx_kickRatingChunksKey,
            self.layer["settings"]["kickRatingChunksKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.ui.cbx_kickRatingChunkWeightsKey,
            self.layer["settings"]["kickRatingChunkWeightsKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.ui.cbx_rateKick,
            self.layer["settings"]["rateKickKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.ui.cbx_hitPointsKey,
            self.layer["settings"]["hitPointsKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.ui.cbx_teamBallPositionKey,
            self.layer["settings"]["teamBallPositionKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.ui.cbx_firstShadowPointKey,
            self.layer["settings"]["firstShadowPointKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.ui.cbx_secondShadowPointKey,
            self.layer["settings"]["secondShadowPointKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.ui.cbx_firstShadowPointAfterKey,
            self.layer["settings"]["firstShadowPointAfterKey"],
            self.nao.debug_data)
        ui_utils.init_cbx(
            self.ui.cbx_secondShadowPointAfterKey,
            self.layer["settings"]["secondShadowPointAfterKey"],
            self.nao.debug_data)

    def accept(self):
        self.layer["name"] = self.ui.edit_name.text()
        self.layer["enabled"] = self.ui.enabledCheckBox.isChecked()
        for key in self.settings_to_ui:
            self.layer["settings"][key] = self.settings_to_ui[key][0]()
        self.update_callback()

    def discard(self):
        self.reset_widgets()
