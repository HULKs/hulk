from dataclasses import dataclass

import mujoco
import mujoco.glfw
import numpy as np
from mujoco import (
    MjData,
    MjModel,
    MjrContext,
    MjrRect,
    MjvCamera,
    MjvOption,
    MjvScene,
)
from numpy.typing import NDArray


def setup_offscreen_rendering_gl_context(
    width: int, height: int
) -> mujoco.glfw.GLContext:
    ctx = mujoco.glfw.GLContext(width, height)
    ctx.make_current()
    return ctx


@dataclass(frozen=True, kw_only=True)
class CameraImage:
    rgb: NDArray[np.uint8]
    depth: NDArray[np.uint16]

    def height(self) -> int:
        return self.rgb.shape[0]

    def width(self) -> int:
        return self.rgb.shape[1]


class CameraRenderer:
    def __init__(
        self,
        *,
        model: MjModel,
        camera_name: str,
        width: int = 640,
        height: int = 480,
        ngeom: int = 250,
    ) -> None:
        self.gl_context = setup_offscreen_rendering_gl_context(width, height)
        self.model = model

        camera = model.camera(camera_name)
        self.camera = MjvCamera()
        self.camera.fixedcamid = camera.id
        self.camera.type = mujoco.mjtCamera.mjCAMERA_FIXED.value

        self.scene = MjvScene(model, maxgeom=ngeom)
        self.option = MjvOption()
        self.viewport = MjrRect(0, 0, width, height)
        self.context = MjrContext(
            model, mujoco.mjtFontScale.mjFONTSCALE_150.value
        )

    def render(self, data: MjData) -> CameraImage:
        mujoco.mjv_updateScene(
            self.model,
            data,
            self.option,
            None,
            self.camera,
            mujoco.mjtCatBit.mjCAT_ALL.value,
            self.scene,
        )
        mujoco.mjr_render(self.viewport, self.scene, self.context)

        height = self.viewport.height
        width = self.viewport.width
        rgb_buffer = np.zeros((height, width, 3), dtype=np.uint8)
        zbuffer = np.zeros((height, width), dtype=np.float32)
        mujoco.mjr_readPixels(
            rgb_buffer, zbuffer, self.viewport, self.context
        )

        znear = self.model.vis.map.znear
        zfar = self.model.vis.map.zfar
        depth_buffer = znear / (1 - zbuffer * (1 - znear / zfar))

        return CameraImage(
            rgb=np.flip(rgb_buffer, axis=0),
            depth=np.flip((depth_buffer * 1000).astype(np.uint16), axis=0),
        )
