import mujoco
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


def setup_offscreen_rendering_gl_context(
    width: int, height: int
) -> mujoco.GLContext:
    ctx = mujoco.GLContext(width, height)
    ctx.make_current()
    return ctx


class CameraEncoder:
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

        self.rgb_buffer = np.zeros((height, width, 3), dtype=np.uint8)
        self.depth_buffer = np.zeros((height, width), dtype=np.float32)
        self.combined_buffer = np.zeros((height, width, 4), dtype=np.uint16)

    def render(self, data: MjData) -> bytes | bytearray:
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
        mujoco.mjr_readPixels(
            self.rgb_buffer, self.depth_buffer, self.viewport, self.context
        )
        self.combined_buffer[:, :, :3] = self.rgb_buffer.astype(np.uint16)
        self.combined_buffer[:, :, 3] = (self.depth_buffer * 1000).astype(
            np.uint16
        )
        return self.rgb_buffer
        # return imagecodecs.jpegxl_encode(
        #     self.combined_buffer, lossless=True, level=7
        # )
        return b"data"
