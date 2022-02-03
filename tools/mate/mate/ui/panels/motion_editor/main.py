import uuid
import json
import os
import typing

import PyQt5.QtCore as qtc
import PyQt5.QtWidgets as qtw
import PyQt5.QtGui as qtg

from mate.net.nao import Nao
import mate.net.utils as netutils
import mate.net.nao_data as nao_data
import mate.ui.utils as ui_utils

from .motion_editor import *
from .render import RenderView
from .config import Config
from mate.ui.panels._panel import _Panel
from mate.debug.colorlog import ColorLog

logger = ColorLog()


class Main(_Panel):
    name = "Motion Editor"

    def __init__(self, main_window, nao: Nao, model: typing.Dict = None):
        super(Main, self).__init__(main_window, self.name, nao)
        ui_utils.loadUi(__file__, self)
        self.model = ui_utils.load_model(
            os.path.dirname(__file__) + "/model.json", model)

        self.play_timer = qtc.QTimer()
        self.capture_timer = qtc.QTimer()

        # Initialize model to assure consistent state
        reset_model(self.model)

        # Rendered View
        self.render_view = RenderView(self.model, self)
        format = qtg.QSurfaceFormat.defaultFormat()
        format.setSamples(4)
        self.render_view.setFormat(format)
        self.config_view = Config(self.model)
        self.tabWidget_left.addTab(self.render_view, "Display")
        self.tabWidget_left.addTab(self.config_view, "Config")

        # Load motion-file
        if self.model["opened_file"] != "": 
            self.open_motion2file(self.model["opened_file"])

        # Connect GUI elements to functions
        self.connect_gui()

        if self.nao.is_connected():
            self.connect(self.nao)

    def connect(self, nao: Nao):
        self.nao = nao
        self.nao.debug_protocol.subscribe_status(
            netutils.ConnectionStatusType.connection_lost, self.identifier,
            self.connection_lost)
        if self.model["motion2_data"] is not None:
            self.set_connected_gui(True)
        self.nao.config_protocol.subscribe(
            self.model["behavior_module_key"],
            self.identifier, lambda d: self.get_behavior_module_data(d))

    def disconnect(self):
        if self.model["live_mode"]:
            self.disconnect_live_mode()
        if self.model["puppet_mode"]:
            self.disconnect_puppet_mode()
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe_status(
                netutils.ConnectionStatusType.connection_lost,
                self.identifier)
            self.nao.config_protocol.unsubscribe(
                self.model["behavior_module_key"],
                self.identifier)

    def connection_lost(self):
        if self.model["live_mode"]:
            self.disconnect_live_mode()
        if self.model["puppet_mode"]:
            self.disconnect_puppet_mode()
        self.set_connected_gui(False)

    def unsubscribe(self):
        if self.nao.is_connected():
            self.nao.config_protocol.unsubscribe(
                self.model["behavior_module_key"], self.identifier)

    def closeEvent(self, event):
        self.disconnect()
        self.deleteLater()
        super(Main, self).closeEvent(event)


    def connect_gui(self):
        # File operations
        self.btn_new.clicked.connect(lambda: self.new_motion2file())
        self.btn_open.clicked.connect(lambda: self.open_motion2file())
        self.btn_save.clicked.connect(lambda: self.save_motion2file())
        self.edit_title.textEdited.connect(lambda: self.title_changed())

        # Edit-mode xor Live-mode toggle
        self.btn_edit_mode.toggled.connect(self.live_mode_toggle)

        # Edit controls
        self.slider_timeline.valueChanged.connect(self.timeline_moved)
        self.spin_duration.valueChanged.connect(self.duration_changed)
        self.play_timer.timeout.connect(self.play_step)
        self.btn_play.clicked.connect(self.play_toggle)
        self.spin_speed.valueChanged.connect(self.speed_changed)
        self.btn_loop.toggled.connect(self.loop_toggle)

        # Frame operations
        self.spin_frame_time.valueChanged.connect(self.time_changed)
        self.slider_frames.valueChanged.connect(self.frame_slider_moved)
        self.tbl_joints.itemChanged.connect(self.joint_item_changed)
        self.tbl_joints.itemSelectionChanged.connect(self.joint_selection_changed)
        self.btn_add.clicked.connect(self.add_frame_to_motion)
        self.btn_add_pose.clicked.connect(self.add_pose_to_motion)
        self.btn_duplicate.clicked.connect(self.duplicate_frame)
        self.btn_cut.clicked.connect(self.cut_frame)
        self.btn_delete.clicked.connect(self.delete_frame)

        # Puppet mode controls
        self.btn_puppet_mode.clicked.connect(self.puppet_mode_toggle)
        
        # Live mode controls
        self.capture_timer.timeout.connect(self.capture_step)
        self.btn_capture.clicked.connect(self.capture_toggle)
        self.spin_delta_threshold.valueChanged.connect(self.capture_delta_threshold_changed)
        self.spin_capture_FPS.valueChanged.connect(self.capture_fps_changed)

    def set_gui_enabled(self, b):
        self.btn_save.setEnabled(b)
        self.lbl_frameCount.setEnabled(b)
        self.lbl_frame_time.setEnabled(b)
        self.spin_frame_time.setEnabled(b)
        self.slider_frames.setEnabled(b)
        self.tbl_joints.setEnabled(b)
        self.btn_add.setEnabled(b)
        self.btn_add_pose.setEnabled(b)
        self.btn_duplicate.setEnabled(b)
        self.btn_cut.setEnabled(b)
        self.btn_delete.setEnabled(b)
        self.slider_timeline.setEnabled(b)
        self.btn_edit_mode.setEnabled(b)
        self.btn_play.setEnabled(b)
        self.lbl_duration.setEnabled(b)
        self.spin_duration.setEnabled(b)
        self.lbl_speed.setEnabled(b)
        self.spin_speed.setEnabled(b)
        self.btn_loop.setEnabled(b)

    def connect_live_mode_gui(self, b):
        self.btn_edit_mode.setChecked(not b)
        self.btn_capture.setEnabled(b)
        self.lbl_capture_FPS.setEnabled(b)
        self.spin_capture_FPS.setEnabled(b)
        self.lbl_delta_threshold.setEnabled(b)
        self.spin_delta_threshold.setEnabled(b)
        self.tbl_joints.setEnabled(not b)
        self.btn_loop.setEnabled(not b)
        self.btn_play.setEnabled(not b)
        self.lbl_speed.setEnabled(not b)
        self.spin_speed.setEnabled(not b)
        self.lbl_duration.setEnabled(not b)
        self.spin_duration.setEnabled(not b)

    def set_gui_to_model(self):
        self.blockSignals(True)
        m = self.model
        m2d = self.model["motion2_data"]
        current_frame = m2d["position"][m["current_frame"]]
        self.edit_title.setText(m2d["header"]["title"])
        self.slider_timeline.setMaximum(m2d["header"]["time"])
        self.spin_duration.setValue(m2d["header"]["time"])
        self.slider_timeline.setValue(m["t_to_reach_duration"])
        self.slider_frames.setMaximum(len(m2d["position"]))
        self.slider_frames.setValue(m["current_frame"]+1)
        self.lbl_frame_time.setText(
            str(int(self.model["t_to_reach_current_frame"] *
                    current_frame["time"])) + " ms of")
        self.spin_frame_time.setValue(current_frame["time"])
        self.lbl_frameCount.setText("Frame {} of {}".format(
            m["current_frame"] + 1,
            len(m2d["position"])))
        self.lbl_duration.setText(
            str(m["t_to_reach_duration"]) + " ms of")
        self.btn_loop.setChecked(m["loop"])
        self.btn_puppet_mode.setChecked(m["puppet_mode"])
        self.spin_speed.setValue(m["speed"])
        self.spin_capture_FPS.setValue(m["capture_fps"])
        self.spin_delta_threshold.setValue(m["delta_threshold"])
        self.tbl_joints.clear()
        self.tbl_joints.setRowCount(len(m2d["header"]["joints"]))
        self.tbl_joints.setColumnCount(2)
        self.tbl_joints.setHorizontalHeaderLabels(["joint name", "angle"])
        for index, value in enumerate(current_frame["parameters"]):
            joint = Joints(m2d["header"]["joints"][index])
            self.tbl_joints.setVerticalHeaderItem(
                index, qtw.QTableWidgetItem(joint.value.__str__()))
            item = qtw.QTableWidgetItem()
            item.setFlags(qtc.Qt.ItemIsSelectable | qtc.Qt.ItemIsEnabled)
            item.setData(qtc.Qt.DisplayRole, joint.name)
            self.tbl_joints.setItem(index, 0, item)
            item = qtw.QTableWidgetItem()
            item.setData(qtc.Qt.DisplayRole, float(value))
            self.tbl_joints.setItem(index, 1, item)
            if index == m["highlight_joint_plot"]:
                item.setSelected(True)
        self.connect_live_mode_gui(m["live_mode"])
        self.blockSignals(False)

    def update_highlighted_joint_table_item(self):
        if self.model["valid"]:
            index = self.model["highlight_joint_plot"]
            value = self.model["motion2_data"]["position"][self.model["current_frame"]]["parameters"][index]
            self.tbl_joints.blockSignals(True)
            self.tbl_joints.item(index, 1).setData(qtc.Qt.DisplayRole, value)
            self.tbl_joints.blockSignals(False)

    def set_connected_gui(self, b):
        self.btn_live_mode.setEnabled(b)
        self.btn_puppet_mode.setEnabled(b)

    def play_toggle(self):
        if self.model["is_playing"]:
            self.stop_playing()
        else:
            self.start_playing()

    def loop_toggle(self):
        logger.debug(__name__ + ": Setting loop playback to " + str(self.btn_loop.isChecked()))
        self.model["loop"] = self.btn_loop.isChecked()

    def speed_changed(self):
        self.model["speed"] = self.spin_speed.value()

    def start_playing(self):
        logger.debug(__name__ + ": Start playing motion " + self.edit_title.text())
        end_time = self.model["motion2_data"]["header"]["time"]
        if self.model["t_to_reach_duration"] == end_time:    
            self.model["t_to_reach_duration"] = 0
        self.model["is_playing"] = True
        self.btn_play.setText("Pause")
        self.play_timer.start(1000 / self.model["playback_fps"])

    def play_step(self):
        total_time = self.model["motion2_data"]["header"]["time"]
        self.model["t_to_reach_duration"] += int((self.model["speed"]/100.0) * 1000 /self.model["playback_fps"])
        if self.model["t_to_reach_duration"] >= total_time:
            if self.model["loop"]:
                self.model["t_to_reach_duration"] = 0
            else:
                self.model["t_to_reach_duration"] = total_time
                self.stop_playing()
        frames = self.model["motion2_data"]["position"]
        frame_index = 0
        t_accumulator = frames[0]["time"]
        frame_count = len(frames)
        while t_accumulator < self.model["t_to_reach_duration"] and frame_index < frame_count:
            frame_index += 1
            if frame_index > len(frames) - 1:
                frame_index = len(frames) - 1
                t_accumulator = self.model["t_to_reach_duration"]
            else:
                t_accumulator += frames[frame_index]["time"]
        t = ((t_accumulator - self.model["t_to_reach_duration"]) / frames[frame_index]["time"])
        self.model["t_to_reach_current_frame"] = 1.0 - t
        if self.model["t_to_reach_current_frame"] > 1.0:
            self.model["t_to_reach_current_frame"] = 1.0
        send_frame = False
        if self.model["current_frame"] != int(frame_index):
            send_frame = True
        self.model["current_frame"] = int(frame_index)
        self.set_gui_to_model()
        self.render_view.update()
        if self.model["puppet_mode"] and send_frame:
            self.send_current_frame()

    def stop_playing(self):
        logger.debug(__name__ + ": Stop playing motion " + self.edit_title.text())
        self.model["is_playing"] = False
        self.btn_play.setText("Play")
        self.play_timer.stop()

    # Motion capturing from Robot
    def capture_toggle(self):
        if self.model["is_capturing"]:
            self.stop_capturing()
        else:
            self.start_capturing()

    def start_capturing(self):
        logger.debug(__name__ + ": Start capture mode")
        self.frame_time_accumulator = 0
        self.model["is_capturing"] = True
        self.btn_capture.setText("Stop Capture")
        self.capture_timer.start(1000 / self.model["capture_fps"])

    def capture_step(self):
        if get_delta(self.model) < self.model["delta_threshold"]:
            self.frame_time_accumulator += int(1000 / self.model["capture_fps"])
        else:
            frame = {}
            frame["time"] = int(1000 / self.model["capture_fps"]) + self.frame_time_accumulator
            self.frame_time_accumulator = 0
            frame["parameters"] = self.model["live_angles"]
            self.model["motion2_data"]["header"]["time"] += frame["time"]
            self.model["motion2_data"]["position"].insert(
                self.model["current_frame"] + 1, frame)
            self.model["current_frame"] += 1
            self.model["t_to_reach_duration"] += frame["time"]
            self.model["t_to_reach_current_frame"] = 1.0
            self.set_gui_to_model()
            self.render_view.update()

    def stop_capturing(self):
        logger.debug(__name__ + ": Stop capture mode")
        self.model["is_capturing"] = False
        self.btn_capture.setText("Start Capture")
        self.capture_timer.stop()

    # Rig to Robot mode
    def puppet_mode_toggle(self):
        if self.nao.is_connected():
            if self.btn_puppet_mode.isChecked():
                logger.debug(__name__ + ": Start puppet mode")
                self.model["puppet_mode"] = True
                self.nao.config_protocol.set(self.model["behavior_module_key"],
                                             "enableRemotePuppetMode", True)
                self.send_current_frame()
            else:
                self.disconnect_puppet_mode()

    def get_behavior_module_data(self, data: nao_data.ConfigMount):
        if data.data["enableRemotePuppetMode"]:
            self.model["puppet_mode"] = True
            self.btn_puppet_mode.setChecked(True)
            self.send_current_frame()
        else:
            self.model["puppet_mode"] = False
            self.btn_puppet_mode.setChecked(False)

    def send_current_frame(self):
        if self.nao.is_connected() and self.model["valid"]:
            value = {
                "jointAngles":
                self.model["motion2_data"]["position"][self.model["current_frame"]]["parameters"],
                #get_current_angles_sorted(self.model),
                "interpolationTime":
                self.model["motion2_data"]["position"][self.model["current_frame"]]["time"] / 1000
                #get_absolute_frame_time(self.model,
                #                        self.model["current_frame"])
            }
            self.nao.config_protocol.set(self.model["puppet_key"],
                                         "remotePuppetJointKeyFrame", value)

    def disconnect_puppet_mode(self):
        logger.debug(__name__ + ": Stop puppet mode")
        self.model["puppet_mode"] = False
        if self.nao.is_connected():
            self.nao.config_protocol.set(self.model["behavior_module_key"],
                                         "enableRemotePuppetMode", False)

    def live_mode_toggle(self):
        if self.nao.is_connected():
            if self.btn_live_mode.isChecked():
                logger.debug(__name__ + ": Switch to live-mode")
                if self.model["puppet_mode"]:
                    self.disconnect_puppet_mode()
                    self.btn_puppet_mode.setChecked(False)
                self.connect_live_mode()
            else:
                logger.debug(__name__ + ": Switch to edit-mode")
                self.disconnect_live_mode()

    def connect_live_mode(self):
        if self.model["is_playing"]:
            self.stop_playing()
        self.model["live_mode"] = True
        self.nao.debug_protocol.subscribe(self.model["live_angle_key"],
                                          self.identifier,
                                          self.update_live_angles)
        self.connect_live_mode_gui(True)

    def update_live_angles(self, data: netutils.Data):
        self.model["live_angles"] = data.data["angles"]
        self.render_view.update()

    def disconnect_live_mode(self):
        self.model["live_mode"] = False
        if self.nao.is_connected():
            self.nao.debug_protocol.unsubscribe(self.model["live_angle_key"],
                                                self.identifier)
        self.connect_live_mode_gui(False)
        self.model["t_to_reach_current_frame"] = 0.0
        self.model["t_to_reach_duration"] = 0
        self.set_gui_to_model()
        self.render_view.update()

    def duration_changed(self):
        if self.spin_duration.value() >= 10:
            self.model["motion2_data"]["header"]["time"] = self.spin_duration.value()
            calculate_frame_durations(self.model)
            calculate_motion_duration(self.model)
            self.set_gui_to_model()
            self.render_view.update()
            #if self.model["puppet_mode"]:
            #    self.send_current_frame()

    # Frame related
    def time_changed(self):
        if self.spin_frame_time.value() >= 10:
            self.model["motion2_data"]["position"][self.model["current_frame"]]["time"] = self.spin_frame_time.value()
            calculate_motion_duration(self.model)
            self.set_gui_to_model()
            self.render_view.update()
            #if self.model["puppet_mode"]:
            #    self.send_current_frame()

    # Joint angle modding
    def joint_item_changed(self, item: qtw.QTableWidgetItem):
        if self.model["valid"] and self.tbl_joints.currentItem() is not None:
            joint_index = self.tbl_joints.currentItem().row()
            self.model["motion2_data"]["position"][
                self.model["current_frame"]]["parameters"][
                    joint_index] = item.data(qtc.Qt.DisplayRole)
            if not self.model["live_mode"]:
                self.render_view.update()
            if self.model["puppet_mode"]:
                self.send_current_frame()

    def joint_selection_changed(self):
        if self.model["valid"] and self.tbl_joints.currentItem() is not None:
            joint_index = self.tbl_joints.currentItem().row()
            self.model["selected_joint"] = Joints(
                self.model["motion2_data"]["header"]["joints"][
                    joint_index])
            self.model["highlight_joint_plot"] = joint_index
            self.render_view.update()

    def add_frame_to_motion(self):
        logger.debug(__name__ + ": Add frame to motion")
        self.model["valid"] = False
        add_frame(self.model)
        calculate_motion_duration(self.model)
        self.set_gui_to_model()
        self.model["valid"] = True
        self.render_view.update()
        if self.model["puppet_mode"]:
            self.send_current_frame()

    def add_pose_to_motion(self):
        logger.debug(__name__ + ": Add pose to motion")
        self.model["valid"] = False
        pose_file_path = qtw.QFileDialog.getOpenFileName(
                self, "Add pose file",
                os.getcwd() + "/../../etc/poses")[0]
        if pose_file_path == '':
            return
        angles = []
        try:
            f = open(pose_file_path, 'r')
            angles = f.readlines()
            f.close()
        except Exception as e:
            self.window().statusBar().showMessage(str(e))
            return
        angles = [x.strip() for x in angles]
        for x in angles:
            if x == "":
                angles.remove(x)
        angles = [float(x) for x in angles]
        add_frame(self.model, angles_to_copy=angles)
        calculate_motion_duration(self.model)
        self.set_gui_to_model()
        self.model["valid"] = True
        self.render_view.update()
        if self.model["puppet_mode"]:
            self.send_current_frame()

    def duplicate_frame(self):
        if not self.model["valid"]:
            return
        logger.debug(__name__ + ": Duplicate current frame")
        self.model["valid"] = False
        add_frame(self.model, get_current_position(self.model)["parameters"])
        calculate_motion_duration(self.model)
        self.set_gui_to_model()
        self.model["valid"] = True
        self.render_view.update()
        #if self.model["puppet_mode"]:
        #    self.send_current_frame()

    def cut_frame(self):
        if not self.model["valid"]:
            return
        t = self.model["t_to_reach_current_frame"]
        if t not in [0.0,1.0]:
            logger.debug(__name__ + ": Cut current frame")
            self.model["valid"] = False
            cut_frame_at_t(self.model)
            self.set_gui_to_model()
            self.model["valid"] = True
            self.render_view.update()
            if self.model["puppet_mode"]:
                self.send_current_frame()

    def delete_frame(self):
        if not self.model["valid"]:
            return
        self.model["valid"] = False
        self.model["motion2_data"]["position"].pop(self.model["current_frame"])
        self.model["current_frame"] = max(0, self.model["current_frame"] - 1)
        if len(self.model["motion2_data"]["position"]):
            self.model["valid"] = True
            calculate_motion_duration(self.model)
            self.model["t_to_reach_current_frame"] = 1.0
            self.model["t_to_reach_duration"] = 0
            for i in range(self.model["current_frame"] + 1):
                self.model["t_to_reach_duration"] += self.model["motion2_data"]["position"][i]["time"]
            self.set_gui_to_model()
            self.model["valid"] = True
            self.render_view.update()
            if self.model["puppet_mode"]:
                self.send_current_frame()
        else:
            self.set_gui_enabled(False)
            self.render_view.update()

    # Motion operations
    def new_motion2file(self):
        reset_model(self.model)
        add_frame(self.model)
        calculate_motion_duration(self.model)
        self.set_gui_to_model()
        self.model["valid"] = True
        self.set_gui_enabled(True)
        self.render_view.update()
        self.set_connected_gui(self.nao.is_connected())

    def open_motion2file(self, motion2_file_path: str = None):
        if motion2_file_path is None:
            motion2_file_path = qtw.QFileDialog.getOpenFileName(
                self, "Open motion2 file",
                os.getcwd() + "/../../etc/motions")[0]
        if motion2_file_path == '':
            return
        try:
            f = open(motion2_file_path, 'r')
            data = json.load(f)
            f.close()
        except Exception as e:
            self.window().statusBar().showMessage(str(e))
            return
        reset_model(self.model)
        self.model["opened_file"] = motion2_file_path
        self.model["motion2_data"] = data
        if "commands" in self.model["motion2_data"]:
            self.model["motion2_data"]["position"] = []
            for command in self.model["motion2_data"]["commands"]:
                self.model["motion2_data"]["position"].append(
                    {"time": command["command"]["time"],
                     "parameters": command["command"]["parameters"]})
        self.model["valid"] = True
        calculate_frame_durations(self.model)
        calculate_motion_duration(self.model)
        self.set_gui_to_model()
        self.set_gui_enabled(True)
        self.render_view.update()
        self.connect_live_mode_gui(False)
        self.set_connected_gui(self.nao.is_connected())

    def save_motion2file(self):
        try:
            path_to_file = qtw.QFileDialog.getSaveFileName(
                self, "Save motion2 file",
                os.path.dirname(self.model["opened_file"]) + "/" +
                self.edit_title.text() + ".motion2")[0]
            f = open(path_to_file, 'w')
            json.dump(self.model["motion2_data"], f, indent=4)
            f.write('\n')
            f.close()
            self.window().statusBar().showMessage("Saved motion2 file")
        except Exception as e:
            self.window().statusBar().showMessage(str(e))

    def title_changed(self):
        self.model["motion2_data"]["header"]["title"] = self.edit_title.text()

    def capture_fps_changed(self):
        self.model["capture_fps"] = self.spin_capture_FPS.value()

    def capture_delta_threshold_changed(self):
        self.model["delta_threshold"] = self.spin_delta_threshold.value()

    def frame_slider_moved(self, value):
        if (value - 1 == self.model["current_frame"]):
            return
        duration = 0
        for f in range(value):
            duration += self.model["motion2_data"]["position"][f]["time"]
        self.model["t_to_reach_duration"] = duration
        self.model["current_frame"] = value - 1
        self.model["t_to_reach_current_frame"] = 1.0
        self.set_gui_to_model()
        self.render_view.update()
        if self.model["puppet_mode"]:
            self.send_current_frame()

    def timeline_moved(self, value):
        if (value == self.model["t_to_reach_duration"]):
            return
        self.model["t_to_reach_duration"] = value
        frames = self.model["motion2_data"]["position"]
        frame_index = 0
        t_accumulator = frames[0]["time"]
        frame_count = len(frames)
        while t_accumulator < value and frame_index < frame_count:
            frame_index += 1
            if frame_index > len(frames) - 1:
                frame_index = len(frames) - 1
                t_accumulator = value
            else:
                t_accumulator += frames[frame_index]["time"]
        t = ((t_accumulator - value) / frames[frame_index]["time"])
        self.model["t_to_reach_current_frame"] = 1.0 - t
        if self.model["t_to_reach_current_frame"] > 1.0:
            self.model["t_to_reach_current_frame"] = 1.0
        self.model["current_frame"] = int(frame_index)
        self.set_gui_to_model()
        self.render_view.update()
        if self.model["puppet_mode"]:
            self.send_current_frame()
