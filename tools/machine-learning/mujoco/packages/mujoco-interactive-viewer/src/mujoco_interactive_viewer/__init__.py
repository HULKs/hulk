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

    def _update(self) -> None:
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

            mujoco.mjr_render(self._viewport, self.scene, self.context)

            glfw.swap_buffers(self._window)

        glfw.poll_events()
        # self._render_state.time_per_render = (
        #     0.9 * self._render_state.time_per_render
        #     + 0.1 * (time.time() - render_start)
        # )

    def render(self) -> None:
        if glfw.window_should_close(self._window):
            self.close()
            return

        # self._render_state.loop_count += self.model.opt.timestep / (
        #     self._render_state.time_per_render * self._render_state.run_speed
        # )
        # if self._render_state.render_every_frame:
        self._render_state.loop_count = 1.0
        while self._render_state.loop_count > 0.0:
            self._update()
            self._render_state.loop_count -= 1.0

    def close(self) -> None:
        self.is_alive = False
        glfw.terminate()
        self.context.free()


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
