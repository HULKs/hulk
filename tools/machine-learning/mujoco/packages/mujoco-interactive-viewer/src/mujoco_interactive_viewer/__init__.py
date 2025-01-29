import time
from dataclasses import dataclass
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
    last_mouse_x: int = 0
    last_mouse_y: int = 0


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


@dataclass
class RenderState:
    is_paused: bool = False
    render_every_frame: bool = True
    time_per_render: float = 1.0 / 60.0
    run_speed: float = 1.0
    loop_count: float = 0.0
    steps_to_advance: int = 0


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


class InteractiveViewer:
    def __init__(
        self,
        model: mujoco.MjModel,
        data: mujoco.MjData,
        *,
        title: str = "mujoco-python-viewer",
        width: int | None = None,
        height: int | None = None,
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

        framebuffer_width, framebuffer_height = glfw.get_framebuffer_size(
            self._window
        )

        window_width, _ = glfw.get_window_size(self._window)
        self._scale = framebuffer_width / window_width

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

        self.context = mujoco.MjrContext(
            self.model,
            mujoco.mjtFontScale.mjFONTSCALE_150.value,
        )

        num_figures = 3
        self._figures = []
        for _ in range(num_figures):
            figure = mujoco.MjvFigure()
            mujoco.mjv_defaultFigure(figure)
            figure.flg_extend = 1
            self._figures.append(figure)

        self._viewport = mujoco.MjrRect(
            0,
            0,
            framebuffer_width,
            framebuffer_height,
        )
        self._overlay: dict[mujoco.mjtGridPos, Overlay] = {}
        self._markers = []

    def add_line_to_figure(self, line_name: str, figure_id: int = 0) -> None:
        figure = self._figures[figure_id]
        name_bytes = line_name.encode("utf8")
        if name_bytes == b"":
            raise EmptyLineNameError()
        if name_bytes in figure.linename:
            raise LineNameAlreadyExistsError()

        try:
            empty_line_id = figure.linename.tolist().index(b"")
        except ValueError as e:
            raise NoEmptyLineSlotsError() from e

        figure.linename[empty_line_id] = line_name

        for i in range(mujoco.mjMAXLINEPNT):
            # line data is stored in the form [x0, y0, x1, y1, x2, y2, ...]
            figure.linedata[empty_line_id][2 * i] = -float(i)

    def add_data_to_line(
        self,
        line_name: str,
        line_data: float,
        figure_id: int = 0,
    ) -> None:
        figure = self._figures[figure_id]

        name_bytes = line_name.encode("utf8")
        try:
            line_id = figure.linename.tolist().index(name_bytes)
        except ValueError as e:
            raise LineNotFound() from e

        num_points = figure.linepnt[line_id]
        num_points = min(mujoco.mjMAXLINEPNT, num_points + 1)

        for i in range(num_points - 1, 0, -1):
            figure.linedata[line_id][2 * i + 1] = figure.linedata[line_id][
                2 * i - 1
            ]

        figure.linepnt[line_id] = num_points
        figure.linedata[line_id][1] = line_data

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
                "[S]lower, [F]aster",
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
            "[G]raph Viewer",
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
            "Shad[O]ws",
            "On" if self._visualization_state.show_shadows else "Off",
        )
        add_overlay(
            top_left,
            "T[r]ansparent",
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
                add_overlay(
                    top_left, "Advance simulation by one step", "[right arrow]"
                )
        add_overlay(
            top_left,
            "Toggle geomgroup visibility (0-5)",
            ",".join(
                [
                    "On" if g else "Off"
                    for g in self.visualization_option.geomgroup
                ]
            ),
        )
        add_overlay(
            top_left,
            "Referenc[e] frames",
            mujoco.mjtFrame(self.visualization_option.frame).name,
        )
        add_overlay(top_left, "[H]ide Menus", "")
        add_overlay(top_left, "Cap[t]ure frame", "")

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
                        mujoco.mjtFontScale.mjFONTSCALE_150.value,
                        position.value,
                        self._viewport,
                        overlay.text1,
                        overlay.text2,
                        self.context,
                    )

            if self._visualization_state.show_figures:
                for figure_i, figure in enumerate(self._figures):
                    width_adjustment = self._viewport.width % 4
                    x = 3 * self._viewport.width // 4 + width_adjustment
                    y = figure_i * self._viewport.height // 4
                    viewport = mujoco.MjrRect(
                        x,
                        y,
                        self._viewport.width // 4,
                        self._viewport.height // 4,
                    )

                    has_lines = any(name != b"" for name in figure.linename)
                    if has_lines:
                        mujoco.mjr_figure(viewport, figure, self.context)

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
                # TODO:
                # if self._advance_by_one_step:
                #     self._advance_by_one_step = False
                #     break
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

        # clear markers
        self._markers.clear()

        self._apply_perturbations()

    def close(self) -> None:
        self.is_alive = False
        glfw.terminate()
        self.context.free()

    def _key_callback(
        self,
        window: glfw._GLFWwindow,
        key: int,
        scancode: int,
        action: int,
        mods: int,
    ) -> None:
        _ = window, scancode

        # if action != glfw.RELEASE:
        #     if key == glfw.KEY_LEFT_ALT:
        #         self._hide_menus = False
        #     return
        # # Switch cameras
        # if key == glfw.KEY_TAB:
        #     self.cam.fixedcamid += 1
        #     self.cam.type = mujoco.mjtCamera.mjCAMERA_FIXED
        #     if self.cam.fixedcamid >= self.model.ncam:
        #         self.cam.fixedcamid = -1
        #         self.cam.type = mujoco.mjtCamera.mjCAMERA_FREE
        # # Pause simulation
        # elif key == glfw.KEY_SPACE and self._paused is not None:
        #     self._paused = not self._paused
        # # Advances simulation by one step.
        # elif key == glfw.KEY_RIGHT and self._paused is not None:
        #     self._advance_by_one_step = True
        #     self._paused = True
        # # Slows down simulation
        # elif key == glfw.KEY_S and mods != glfw.MOD_CONTROL:
        #     self._run_speed /= 2.0
        # # Speeds up simulation
        # elif key == glfw.KEY_F:
        #     self._run_speed *= 2.0
        # # Turn off / turn on rendering every frame.
        # elif key == glfw.KEY_D:
        #     self._render_every_frame = not self._render_every_frame
        # # Capture screenshot
        # elif key == glfw.KEY_T:
        #     img = np.zeros(
        #         (
        #             glfw.get_framebuffer_size(self.window)[1],
        #             glfw.get_framebuffer_size(self.window)[0],
        #             3,
        #         ),
        #         dtype=np.uint8,
        #     )
        #     mujoco.mjr_readPixels(img, None, self.viewport, self.ctx)
        #     imageio.imwrite(self._image_path % self._image_idx, np.flipud(img))
        #     self._image_idx += 1
        # # Display contact forces
        # elif key == glfw.KEY_C:
        #     self._contacts = not self._contacts
        #     self.vopt.flags[mujoco.mjtVisFlag.mjVIS_CONTACTPOINT] = (
        #         self._contacts
        #     )
        #     self.vopt.flags[mujoco.mjtVisFlag.mjVIS_CONTACTFORCE] = (
        #         self._contacts
        #     )
        # elif key == glfw.KEY_J:
        #     self._joints = not self._joints
        #     self.vopt.flags[mujoco.mjtVisFlag.mjVIS_JOINT] = self._joints
        # # Display mjtFrame
        # elif key == glfw.KEY_E:
        #     self.vopt.frame += 1
        #     if self.vopt.frame == mujoco.mjtFrame.mjNFRAME.value:
        #         self.vopt.frame = 0
        # # Hide overlay menu
        # elif key == glfw.KEY_LEFT_ALT:
        #     self._hide_menus = True
        # elif key == glfw.KEY_H:
        #     self._hide_menus = not self._hide_menus
        # # Make transparent
        # elif key == glfw.KEY_R:
        #     self._transparent = not self._transparent
        #     if self._transparent:
        #         self.model.geom_rgba[:, 3] /= 5.0
        #     else:
        #         self.model.geom_rgba[:, 3] *= 5.0
        # # Toggle Graph overlay
        # elif key == glfw.KEY_G:
        #     self._hide_graph = not self._hide_graph
        # # Display inertia
        # elif key == glfw.KEY_I:
        #     self._inertias = not self._inertias
        #     self.vopt.flags[mujoco.mjtVisFlag.mjVIS_INERTIA] = self._inertias
        # # Display center of mass
        # elif key == glfw.KEY_M:
        #     self._com = not self._com
        #     self.vopt.flags[mujoco.mjtVisFlag.mjVIS_COM] = self._com
        # # Shadow Rendering
        # elif key == glfw.KEY_O:
        #     self._shadows = not self._shadows
        #     self.scn.flags[mujoco.mjtRndFlag.mjRND_SHADOW] = self._shadows
        # # Convex-Hull rendering
        # elif key == glfw.KEY_V:
        #     self._convex_hull_rendering = not self._convex_hull_rendering
        #     self.vopt.flags[mujoco.mjtVisFlag.mjVIS_CONVEXHULL] = (
        #         self._convex_hull_rendering
        #     )
        # # Wireframe Rendering
        # elif key == glfw.KEY_W:
        #     self._wire_frame = not self._wire_frame
        #     self.scn.flags[mujoco.mjtRndFlag.mjRND_WIREFRAME] = self._wire_frame
        # # Geom group visibility
        # elif key in (
        #     glfw.KEY_0,
        #     glfw.KEY_1,
        #     glfw.KEY_2,
        #     glfw.KEY_3,
        #     glfw.KEY_4,
        #     glfw.KEY_5,
        # ):
        #     self.vopt.geomgroup[key - glfw.KEY_0] ^= 1
        # elif key == glfw.KEY_S and mods == glfw.MOD_CONTROL:
        #     cam_config = {
        #         "type": self.cam.type,
        #         "fixedcamid": self.cam.fixedcamid,
        #         "trackbodyid": self.cam.trackbodyid,
        #         "lookat": self.cam.lookat.tolist(),
        #         "distance": self.cam.distance,
        #         "azimuth": self.cam.azimuth,
        #         "elevation": self.cam.elevation,
        #     }
        #     try:
        #         with open(self.CONFIG_PATH, "w") as f:
        #             yaml.dump(cam_config, f)
        #         print(f"Camera config saved at {self.CONFIG_PATH}")
        #     except Exception as e:
        #         print(e)
        # # Quit
        # if key == glfw.KEY_ESCAPE:
        #     print("Pressed ESC")
        #     print("Quitting.")
        #     glfw.set_window_should_close(self.window, True)
        return

    def _cursor_position_callback(
        self,
        window: glfw._GLFWwindow,
        xpos: int,
        ypos: int,
    ) -> None:
        if not (
            self._interaction_state.left_mouse_button_pressed
            or self._interaction_state.right_mouse_button_pressed
        ):
            return

        mouse_x = int(self._scale * xpos)
        mouse_y = int(self._scale * ypos)

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
        width, height = glfw.get_framebuffer_size(window)

        with self._gui_lock:
            if self.perturbation.active:
                mujoco.mjv_movePerturb(
                    self.model,
                    self.data,
                    action.value,
                    dx / width,
                    dy / height,
                    self.scene,
                    self.perturbation,
                )
            else:
                mujoco.mjv_moveCamera(
                    self.model,
                    action.value,
                    dx / height,
                    dy / height,
                    self.scene,
                    self.camera,
                )

        self._last_mouse_x = mouse_x
        self._last_mouse_y = mouse_y

    def _mouse_button_callback(
        self,
        window: glfw._GLFWwindow,
        button: int,
        action: int,
        mods: int,
    ) -> None:
        self._interaction_state.left_mouse_button_pressed = (
            button == glfw.MOUSE_BUTTON_LEFT and action == glfw.PRESS
        )
        self._interaction_state.right_mouse_button_pressed = (
            button == glfw.MOUSE_BUTTON_RIGHT and action == glfw.PRESS
        )

        x, y = glfw.get_cursor_pos(window)
        self._last_mouse_x = int(self._scale * x)
        self._last_mouse_y = int(self._scale * y)

        # detect a left- or right- doubleclick
        self._interaction_state.left_double_click_pressed = False
        self._interaction_state.right_double_click_pressed = False
        time_now = glfw.get_time()

        if self._interaction_state.left_mouse_button_pressed:
            if self._interaction_state.last_left_click_time is None:
                self._interaction_state.last_left_click_time = glfw.get_time()

            time_diff = time_now - self._interaction_state.last_left_click_time
            if time_diff > 0.01 and time_diff < DOUBLE_CLICK_INTERVAL:
                self._interaction_state.left_double_click_pressed = True
            self._interaction_state.last_left_click_time = time_now

        if self._interaction_state.right_mouse_button_pressed:
            if self._interaction_state.last_right_click_time is None:
                self._interaction_state.last_right_click_time = glfw.get_time()

            time_diff = time_now - self._interaction_state.last_right_click_time
            if time_diff > 0.01 and time_diff < 0.2:
                self._interaction_state.right_double_click_pressed = True
            self._interaction_state.last_right_click_time = time_now

        perturbation = 0
        if mods == glfw.MOD_CONTROL and self.perturbation.select > 0:
            if self._interaction_state.right_mouse_button_pressed:
                perturbation = mujoco.mjtPertBit.mjPERT_TRANSLATE.value
            if self._interaction_state.left_mouse_button_pressed:
                perturbation = mujoco.mjtPertBit.mjPERT_ROTATE.value

            if perturbation and not self.perturbation.active:
                mujoco.mjv_initPerturb(
                    self.model,
                    self.data,
                    self.scene,
                    self.perturbation,
                )
        self.perturbation.active = perturbation

        if (
            self._interaction_state.left_double_click_pressed
            or self._interaction_state.right_double_click_pressed
        ):
            # determine selection mode
            selmode = 0
            if self._interaction_state.left_double_click_pressed:
                selmode = 1
            elif self._interaction_state.right_double_click_pressed:
                selmode = 2
            elif (
                self._interaction_state.right_double_click_pressed
                and mods == glfw.MOD_CONTROL
            ):
                selmode = 3

            width, height = self._viewport.width, self._viewport.height
            aspectratio = width / height
            rel_x = x / width
            rel_y = (self._viewport.height - y) / height
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
            if selmode == 2 or selmode == 3:
                # set cam lookat
                if selbody >= 0:
                    self.camera.lookat = selpnt.flatten()
                # switch to tracking camera if dynamic body clicked
                if selmode == 3 and selbody > 0:
                    self.camera.type = mujoco.mjtCamera.mjCAMERA_TRACKING.value
                    self.camera.trackbodyid = selbody
                    self.camera.fixedcamid = -1
            # set body selection
            else:
                if selbody >= 0:
                    # record selection
                    self.perturbation.select = selbody
                    self.perturbation.skinselect = selskin
                    # compute localpos
                    vec = selpnt.flatten() - self.data.xpos[selbody]
                    mat = self.data.xmat[selbody].reshape(3, 3)
                    self.perturbation.localpos = (
                        self.data.xmat[selbody].reshape(3, 3).dot(vec)
                    )
                else:
                    self.perturbation.select = 0
                    self.perturbation.skinselect = -1
            # stop perturbation on select
            self.perturbation.active = 0

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
