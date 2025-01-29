import time
from dataclasses import dataclass
from enum import Enum
from threading import Lock

import glfw
import mujoco
import numpy as np
from numpy.typing import NDArray

DOUBLE_CLICK_INTERVAL = 0.3


@dataclass
class InteractionState:
    left_mouse_button_pressed: bool = False
    right_mouse_button_pressed: bool = False
    left_double_click_pressed: bool = False
    right_double_click_pressed: bool = False
    last_left_click_time: float | None = None
    last_right_click_time: float | None = None
    last_mouse_x: float = 0.0
    last_mouse_y: float = 0.0

    def detect_click(self, button: int, action: int) -> None:
        self.left_mouse_button_pressed = (
            button == glfw.MOUSE_BUTTON_LEFT and action == glfw.PRESS
        )
        self.right_mouse_button_pressed = (
            button == glfw.MOUSE_BUTTON_RIGHT and action == glfw.PRESS
        )

    def detect_double_click(self) -> None:
        self.left_double_click_pressed = False
        self.right_double_click_pressed = False
        time_now = glfw.get_time()

        if self.left_mouse_button_pressed:
            if self.last_left_click_time is None:
                self.last_left_click_time = glfw.get_time()

            time_diff = time_now - self.last_left_click_time
            if time_diff > 0.01 and time_diff < DOUBLE_CLICK_INTERVAL:
                self.left_double_click_pressed = True
            self.last_left_click_time = time_now

        if self.right_mouse_button_pressed:
            if self.last_right_click_time is None:
                self.last_right_click_time = glfw.get_time()

            time_diff = time_now - self.last_right_click_time
            if time_diff > 0.01 and time_diff < 0.2:
                self.right_double_click_pressed = True
            self.last_right_click_time = time_now


@dataclass
class VisualizationState:
    show_contacts: bool = False
    show_joints: bool = False
    show_figures: bool = True
    transparent: bool = False
    show_shadows: bool = True
    show_wire_frame: bool = False
    show_convex_hull: bool = False
    show_inertias: bool = False
    show_com: bool = False
    show_overlay: bool = True
    show_frame: int = 0

    def toggle_contacts(self, option: mujoco.MjvOption) -> None:
        self.show_contacts = not self.show_contacts
        option.flags[mujoco.mjtVisFlag.mjVIS_CONTACTPOINT] = self.show_contacts
        option.flags[mujoco.mjtVisFlag.mjVIS_CONTACTFORCE] = self.show_contacts

    def toggle_joints(self, option: mujoco.MjvOption) -> None:
        self.show_joints = not self.show_joints
        option.flags[mujoco.mjtVisFlag.mjVIS_JOINT] = self.show_joints

    def cycle_frame_display(self, option: mujoco.MjvOption) -> None:
        self.show_frame += 1
        if self.show_frame == mujoco.mjtFrame.mjNFRAME.value:
            self.show_frame = 0
        option.frame = self.show_frame

    def toggle_overlays(self) -> None:
        self.show_overlay = not self.show_overlay

    def toggle_transparency(self, model: mujoco.MjModel) -> None:
        self.transparent = not self.transparent
        if self.transparent:
            model.geom_rgba[:, 3] /= 5.0
        else:
            model.geom_rgba[:, 3] *= 5.0

    def toggle_figures(self) -> None:
        self.show_figures = not self.show_figures

    def toggle_inertias(self, option: mujoco.MjvOption) -> None:
        self.show_inertias = not self.show_inertias
        option.flags[mujoco.mjtVisFlag.mjVIS_INERTIA] = self.show_inertias

    def toggle_com(self, option: mujoco.MjvOption) -> None:
        self.show_com = not self.show_com
        option.flags[mujoco.mjtVisFlag.mjVIS_COM] = self.show_com

    def toggle_shadows(self, scene: mujoco.MjvScene) -> None:
        self.show_shadows = not self.show_shadows
        scene.flags[mujoco.mjtRndFlag.mjRND_SHADOW] = self.show_shadows

    def toggle_convex_hull(self, option: mujoco.MjvOption) -> None:
        self.show_convex_hull = not self.show_convex_hull
        option.flags[mujoco.mjtVisFlag.mjVIS_CONVEXHULL] = self.show_convex_hull

    def toggle_wire_frame(self, scene: mujoco.MjvScene) -> None:
        self.show_wire_frame = not self.show_wire_frame
        scene.flags[mujoco.mjtRndFlag.mjRND_WIREFRAME] = self.show_wire_frame


@dataclass
class RenderState:
    is_paused: bool = False
    render_every_frame: bool = False
    time_per_render: float = 1.0 / 60.0
    run_speed: float = 1.0
    loop_count: float = 0.0
    steps_to_advance: int = 0

    def toggle_render_every_frame(self) -> None:
        self.render_every_frame = not self.render_every_frame

    def toggle_pause(self) -> None:
        self.is_paused = not self.is_paused

    def advance_by_one_step(self) -> None:
        self.steps_to_advance = 1
        self.is_paused = True

    def run_slower(self) -> None:
        self.run_speed /= 2.0
        if self.run_speed < 2**-4:
            self.run_speed = 2**-4

    def run_faster(self) -> None:
        self.run_speed *= 2.0
        if self.run_speed > 2**4:
            self.run_speed = 2**4


@dataclass
class Marker:
    kind: mujoco.mjtGeom
    size: NDArray[np.float64]
    position: NDArray[np.float64]
    material: NDArray[np.float64]
    rgba: NDArray[np.float32]


@dataclass
class Overlay:
    text1: str
    text2: str

    def __init__(self) -> None:
        self.text1 = ""
        self.text2 = ""


@dataclass
class Figure:
    _figure: mujoco.MjvFigure

    def __init__(self) -> None:
        self._figure = mujoco.MjvFigure()
        mujoco.mjv_defaultFigure(self._figure)
        self._figure.flg_extend = 1

    def add_line(self, line_name: str) -> None:
        name_bytes = line_name.encode("utf8")
        if name_bytes == b"":
            raise EmptyLineNameError()
        if name_bytes in self._figure.linename:
            raise LineNameAlreadyExistsError()

        try:
            empty_line_id = self._figure.linename.tolist().index(b"")
        except ValueError as e:
            raise NoEmptyLineSlotsError() from e

        self._figure.linename[empty_line_id] = line_name

        for i in range(mujoco.mjMAXLINEPNT):
            # line data is stored in the form [x0, y0, x1, y1, x2, y2, ...]
            self._figure.linedata[empty_line_id][2 * i] = -float(i)

    def push_data_to_line(
        self,
        line_name: str,
        line_data: float,
    ) -> None:
        name_bytes = line_name.encode("utf8")
        try:
            line_id = self._figure.linename.tolist().index(name_bytes)
        except ValueError as e:
            raise LineNotFound() from e

        num_points: int = self._figure.linepnt[line_id]  # type: ignore[reportAssignmentType]
        num_points = min(mujoco.mjMAXLINEPNT, num_points + 1)

        for i in range(num_points - 1, 0, -1):
            self._figure.linedata[line_id][2 * i + 1] = self._figure.linedata[
                line_id
            ][2 * i - 1]

        self._figure.linepnt[line_id] = num_points
        self._figure.linedata[line_id][1] = line_data


class InteractiveViewer:
    def __init__(
        self,
        model: mujoco.MjModel,
        data: mujoco.MjData,
        *,
        title: str = "mujoco-python-viewer",
        width: int | None = None,
        height: int | None = None,
        font_scale: mujoco.mjtFontScale = mujoco.mjtFontScale.mjFONTSCALE_100,
    ) -> None:
        self._gui_lock = Lock()
        self._interaction_state = InteractionState()
        self._visualization_state = VisualizationState()
        self._render_state = RenderState()
        self.is_alive = True

        self.model = model
        self.data = data

        glfw.init()

        video_mode = glfw.get_video_mode(glfw.get_primary_monitor())
        if width is None:
            width, _ = video_mode.size
        if height is None:
            _, height = video_mode.size

        self._window = glfw.create_window(width, height, title, None, None)
        glfw.make_context_current(self._window)
        glfw.swap_interval(1)

        glfw.set_cursor_pos_callback(
            self._window, self._cursor_position_callback
        )
        glfw.set_mouse_button_callback(
            self._window,
            self._mouse_button_callback,
        )
        glfw.set_scroll_callback(self._window, self._scroll_callback)
        glfw.set_key_callback(self._window, self._key_callback)

        self.visualization_option = mujoco.MjvOption()
        self.camera = mujoco.MjvCamera()
        self.scene = mujoco.MjvScene(self.model, maxgeom=10000)
        self.perturbation = mujoco.MjvPerturb()

        self.context = mujoco.MjrContext(self.model, font_scale.value)
        self._font_scale = font_scale

        framebuffer_width, framebuffer_height = glfw.get_framebuffer_size(
            self._window
        )
        self._viewport = mujoco.MjrRect(
            0,
            0,
            framebuffer_width,
            framebuffer_height,
        )
        self._markers: list[Marker] = []
        self._figures: dict[str, Figure] = {}
        self._overlay: dict[mujoco.mjtGridPos, Overlay] = {}

    def add_marker(self, marker: Marker) -> None:
        self._markers.append(marker)

    def _add_marker_to_scene(self, marker: Marker) -> None:
        if self.scene.ngeom >= self.scene.maxgeom:
            raise OutOfGeomsError()

        geom = self.scene.geoms[self.scene.ngeom]
        mujoco.mjv_initGeom(
            geom,
            type=marker.kind.value,
            size=marker.size,
            pos=marker.position,
            mat=marker.material,
            rgba=marker.rgba,
        )
        self.scene.ngeom += 1

    def figure(self, name: str) -> Figure:
        if name not in self._figures:
            self._figures[name] = Figure()
        return self._figures[name]

    def _create_overlay(self) -> None:
        top_left = mujoco.mjtGridPos.mjGRID_TOPLEFT
        bottom_left = mujoco.mjtGridPos.mjGRID_BOTTOMLEFT

        def add_overlay(
            position: mujoco.mjtGridPos,
            text1: str,
            text2: str,
        ) -> None:
            if position not in self._overlay:
                self._overlay[position] = Overlay()
            self._overlay[position].text1 += text1 + "\n"
            self._overlay[position].text2 += text2 + "\n"

        if self._render_state.render_every_frame:
            add_overlay(top_left, "", "")
        else:
            add_overlay(
                top_left,
                f"Run speed = {self._render_state.run_speed:.3f}x real time",
                "Slower [<], Faster [>]",
            )
        add_overlay(
            top_left,
            "Ren[d]er every frame",
            "On" if self._render_state.render_every_frame else "Off",
        )
        add_overlay(
            top_left,
            f"Switch camera (#cams = {self.model.ncam + 1})",
            f"[Tab] (camera ID = {self.camera.fixedcamid})",
        )
        add_overlay(
            top_left,
            "[C]ontact forces",
            "On" if self._visualization_state.show_contacts else "Off",
        )
        add_overlay(
            top_left,
            "[J]oints",
            "On" if self._visualization_state.show_joints else "Off",
        )
        add_overlay(
            top_left,
            "Show Figures [P]",
            "Off" if self._visualization_state.show_figures else "On",
        )
        add_overlay(
            top_left,
            "[I]nertia",
            "On" if self._visualization_state.show_inertias else "Off",
        )
        add_overlay(
            top_left,
            "Center of [M]ass",
            "On" if self._visualization_state.show_com else "Off",
        )
        add_overlay(
            top_left,
            "[S]hadows",
            "On" if self._visualization_state.show_shadows else "Off",
        )
        add_overlay(
            top_left,
            "[T]ransparent",
            "On" if self._visualization_state.transparent else "Off",
        )
        add_overlay(
            top_left,
            "[W]ireframe",
            "On" if self._visualization_state.show_wire_frame else "Off",
        )
        add_overlay(
            top_left,
            "Con[V]ex Hull Rendering",
            "On" if self._visualization_state.show_convex_hull else "Off",
        )
        if self._render_state.is_paused is not None:
            if not self._render_state.is_paused:
                add_overlay(top_left, "Stop", "[Space]")
            else:
                add_overlay(top_left, "Start", "[Space]")
                add_overlay(top_left, "Advance simulation by one step", "[.]")
        add_overlay(
            top_left,
            "Reference [F]rames",
            mujoco.mjtFrame(self.visualization_option.frame).name,
        )
        add_overlay(top_left, "[H]ide Overlay", "")

        add_overlay(
            bottom_left,
            "FPS",
            f"{1 / self._render_state.time_per_render}",
        )
        add_overlay(
            bottom_left,
            "Max solver iters",
            str(max(self.data.solver_niter) + 1),
        )
        add_overlay(
            bottom_left,
            "Step",
            str(round(self.data.time / self.model.opt.timestep)),
        )
        add_overlay(bottom_left, "timestep", f"{self.model.opt.timestep:.5f}")

    def _apply_perturbations(self) -> None:
        self.data.xfrc_applied = np.zeros_like(self.data.xfrc_applied)
        mujoco.mjv_applyPerturbPose(
            self.model,
            self.data,
            self.perturbation,
            0,
        )
        mujoco.mjv_applyPerturbForce(
            self.model,
            self.data,
            self.perturbation,
        )

    def _update(self) -> None:
        self._overlay.clear()
        self._create_overlay()

        render_start = time.time()

        self._viewport.width, self._viewport.height = glfw.get_framebuffer_size(
            self._window
        )

        with self._gui_lock:
            mujoco.mjv_updateScene(
                self.model,
                self.data,
                self.visualization_option,
                self.perturbation,
                self.camera,
                mujoco.mjtCatBit.mjCAT_ALL.value,
                self.scene,
            )

            # reset all markers
            for marker in self._markers:
                self._add_marker_to_scene(marker)

            mujoco.mjr_render(self._viewport, self.scene, self.context)

            if self._visualization_state.show_overlay:
                for position, overlay in self._overlay.items():
                    mujoco.mjr_overlay(
                        self._font_scale.value,
                        position.value,
                        self._viewport,
                        overlay.text1,
                        overlay.text2,
                        self.context,
                    )

            if self._visualization_state.show_figures:
                for figure_i, figure in enumerate(self._figures.values()):
                    width_adjustment = self._viewport.width % 4
                    x = 3 * self._viewport.width // 4 + width_adjustment
                    y = figure_i * self._viewport.height // 4
                    viewport = mujoco.MjrRect(
                        x,
                        y,
                        self._viewport.width // 4,
                        self._viewport.height // 4,
                    )

                    has_lines = any(
                        name != b"" for name in figure._figure.linename
                    )
                    if has_lines:
                        mujoco.mjr_figure(
                            viewport,
                            figure._figure,
                            self.context,
                        )

            glfw.swap_buffers(self._window)

        glfw.poll_events()
        self._render_state.time_per_render = (
            0.9 * self._render_state.time_per_render
            + 0.1 * (time.time() - render_start)
        )

    def render(self) -> None:
        if not self.is_alive:
            raise NoWindowError()

        if glfw.window_should_close(self._window):
            self.close()
            return

        if self._render_state.is_paused:
            while self._render_state.is_paused:
                self._update()
                if glfw.window_should_close(self._window):
                    self.close()
                    break
                if self._render_state.steps_to_advance > 0:
                    self._render_state.steps_to_advance -= 1
                    break
        else:
            self._render_state.loop_count += self.model.opt.timestep / (
                self._render_state.time_per_render
                * self._render_state.run_speed
            )
            if self._render_state.render_every_frame:
                self._render_state.loop_count = 1.0
            while self._render_state.loop_count > 0.0:
                self._update()
                self._render_state.loop_count -= 1.0

        self._markers.clear()

        self._apply_perturbations()

    def close(self) -> None:
        self.is_alive = False
        glfw.terminate()
        self.context.free()

    def _key_callback(  # noqa: C901
        self,
        window: glfw._GLFWwindow,
        key: int,
        scancode: int,
        action: int,
        mods: int,
    ) -> None:
        _ = window, scancode

        if action != glfw.RELEASE:
            return

        if key == glfw.KEY_TAB:
            self._cycle_cameras()
        elif key == glfw.KEY_SPACE:
            self._render_state.toggle_pause()
        elif key == glfw.KEY_PERIOD:
            self._render_state.advance_by_one_step()
        elif key == glfw.KEY_COMMA and mods == glfw.MOD_SHIFT:
            self._render_state.run_slower()
        elif key == glfw.KEY_PERIOD and mods == glfw.MOD_SHIFT:
            self._render_state.run_faster()
        elif key == glfw.KEY_D:
            self._render_state.toggle_render_every_frame()
        elif key == glfw.KEY_C:
            self._visualization_state.toggle_contacts(self.visualization_option)
        elif key == glfw.KEY_J:
            self._visualization_state.toggle_joints(self.visualization_option)
        elif key == glfw.KEY_F:
            self._visualization_state.cycle_frame_display(
                self.visualization_option
            )
        elif key == glfw.KEY_H:
            self._visualization_state.toggle_overlays()
        elif key == glfw.KEY_T:
            self._visualization_state.toggle_transparency(self.model)
        elif key == glfw.KEY_P:
            self._visualization_state.toggle_figures()
        elif key == glfw.KEY_I:
            self._visualization_state.toggle_inertias(self.visualization_option)
        elif key == glfw.KEY_M:
            self._visualization_state.toggle_com(self.visualization_option)
        elif key == glfw.KEY_S:
            self._visualization_state.toggle_shadows(self.scene)
        elif key == glfw.KEY_V:
            self._visualization_state.toggle_convex_hull(
                self.visualization_option
            )
        elif key == glfw.KEY_W:
            self._visualization_state.toggle_wire_frame(self.scene)

    def _cursor_position_callback(
        self,
        window: glfw._GLFWwindow,
        mouse_x: float,
        mouse_y: float,
    ) -> None:
        if not (
            self._interaction_state.left_mouse_button_pressed
            or self._interaction_state.right_mouse_button_pressed
        ):
            return

        mod_shift = (
            glfw.get_key(window, glfw.KEY_LEFT_SHIFT) == glfw.PRESS
            or glfw.get_key(window, glfw.KEY_RIGHT_SHIFT) == glfw.PRESS
        )
        if self._interaction_state.right_mouse_button_pressed:
            action = (
                mujoco.mjtMouse.mjMOUSE_MOVE_H
                if mod_shift
                else mujoco.mjtMouse.mjMOUSE_MOVE_V
            )
        elif self._interaction_state.left_mouse_button_pressed:
            action = (
                mujoco.mjtMouse.mjMOUSE_ROTATE_H
                if mod_shift
                else mujoco.mjtMouse.mjMOUSE_ROTATE_V
            )
        else:
            action = mujoco.mjtMouse.mjMOUSE_ZOOM

        dx = mouse_x - self._interaction_state.last_mouse_x
        dy = mouse_y - self._interaction_state.last_mouse_y
        window_width, window_height = glfw.get_window_size(window)

        with self._gui_lock:
            if self.perturbation.active:
                mujoco.mjv_movePerturb(
                    self.model,
                    self.data,
                    action.value,
                    dx / window_width,
                    dy / window_height,
                    self.scene,
                    self.perturbation,
                )
            else:
                mujoco.mjv_moveCamera(
                    self.model,
                    action.value,
                    dx / window_width,
                    dy / window_height,
                    self.scene,
                    self.camera,
                )

        self._interaction_state.last_mouse_x = mouse_x
        self._interaction_state.last_mouse_y = mouse_y

    def _mouse_button_callback(
        self,
        window: glfw._GLFWwindow,
        button: int,
        action: int,
        mods: int,
    ) -> None:
        self._interaction_state.detect_click(button, action)
        self._interaction_state.detect_double_click()

        mouse_x, mouse_y = glfw.get_cursor_pos(window)
        self._interaction_state.last_mouse_x = mouse_x
        self._interaction_state.last_mouse_y = mouse_y

        self._handle_perturbation(mods)

        self._handle_selection(mods)

        if action == glfw.RELEASE:
            self.perturbation.active = 0

    def _scroll_callback(
        self,
        window: glfw._GLFWwindow,
        x_offset: float,
        y_offset: float,
    ) -> None:
        _ = window, x_offset

        with self._gui_lock:
            mujoco.mjv_moveCamera(
                self.model,
                mujoco.mjtMouse.mjMOUSE_ZOOM.value,
                0.0,
                -0.05 * y_offset,
                self.scene,
                self.camera,
            )

    def _cycle_cameras(self) -> None:
        self.camera.fixedcamid += 1
        self.camera.type = mujoco.mjtCamera.mjCAMERA_FIXED.value
        if self.camera.fixedcamid >= self.model.ncam:
            self.camera.fixedcamid = -1
            self.camera.type = mujoco.mjtCamera.mjCAMERA_FREE.value

    def _handle_perturbation(self, mods: int) -> None:
        perturbation_kind = 0
        is_body_selected = self.perturbation.select > 0
        if mods == glfw.MOD_CONTROL and is_body_selected:
            if self._interaction_state.right_mouse_button_pressed:
                perturbation_kind = mujoco.mjtPertBit.mjPERT_TRANSLATE.value
            if self._interaction_state.left_mouse_button_pressed:
                perturbation_kind = mujoco.mjtPertBit.mjPERT_ROTATE.value

            if perturbation_kind and not self.perturbation.active:
                mujoco.mjv_initPerturb(
                    self.model,
                    self.data,
                    self.scene,
                    self.perturbation,
                )
        self.perturbation.active = perturbation_kind

    def _handle_selection(self, mods: int) -> None:
        class Mode(Enum):
            Select = 1
            LookAt = 2
            Track = 3

        if (
            self._interaction_state.left_double_click_pressed
            or self._interaction_state.right_double_click_pressed
        ):
            # determine selection mode
            mode = None
            if self._interaction_state.left_double_click_pressed:
                mode = Mode.Select
            elif self._interaction_state.right_double_click_pressed:
                mode = Mode.LookAt
            elif (
                self._interaction_state.right_double_click_pressed
                and mods == glfw.MOD_CONTROL
            ):
                mode = Mode.Track

            mouse_x, mouse_y = glfw.get_cursor_pos(self._window)
            window_width, window_height = glfw.get_window_size(self._window)
            aspectratio = window_width / window_height
            rel_x = mouse_x / window_width
            rel_y = (window_height - mouse_y) / window_height
            selpnt = np.zeros((3, 1), dtype=np.float64)
            selgeom = np.zeros((1, 1), dtype=np.int32)
            selflex = np.zeros((1, 1), dtype=np.int32)
            selskin = np.zeros((1, 1), dtype=np.int32)

            selbody = mujoco.mjv_select(
                self.model,
                self.data,
                self.visualization_option,
                aspectratio,
                rel_x,
                rel_y,
                self.scene,
                selpnt,
                selgeom,
                selflex,
                selskin,
            )

            # set lookat point, start tracking is requested
            if mode == Mode.LookAt or mode == Mode.Track:
                self.camera.lookat = selpnt.flatten()

            if mode == Mode.Track:
                self.camera.type = mujoco.mjtCamera.mjCAMERA_TRACKING.value
                self.camera.trackbodyid = selbody
                self.camera.fixedcamid = -1

            if mode == Mode.Select:
                if selbody >= 0:
                    # record selection
                    self.perturbation.select = selbody
                    self.perturbation.skinselect = selskin[0]
                    # compute localpos
                    vec = selpnt.flatten() - self.data.xpos[selbody]
                    mat = self.data.xmat[selbody].reshape(3, 3)
                    self.perturbation.localpos = mat.dot(vec)
                else:
                    self.perturbation.select = 0
                    self.perturbation.skinselect = -1
            # stop perturbation on select
            self.perturbation.active = 0


class EmptyLineNameError(Exception):
    def __init__(self, message: str = "line name cannot be empty") -> None:
        super().__init__(message)


class LineNameAlreadyExistsError(Exception):
    def __init__(
        self,
        message: str = "line name already exists in this figure",
    ) -> None:
        super().__init__(message)


class NoEmptyLineSlotsError(Exception):
    def __init__(
        self,
        message: str = "no empty line slots available",
    ) -> None:
        super().__init__(message)


class LineNotFound(Exception):
    def __init__(
        self,
        message: str = "line not found in this figure",
    ) -> None:
        super().__init__(message)


class OutOfGeomsError(Exception):
    def __init__(
        self,
        message: str = "Ran out of geoms",
    ) -> None:
        super().__init__(message)


class NoWindowError(Exception):
    def __init__(
        self,
        message: str = "No window to render to",
    ) -> None:
        super().__init__(message)
