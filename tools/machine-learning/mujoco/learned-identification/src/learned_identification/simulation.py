from collections.abc import Sequence

import mediapy as media
import mujoco as mj
import numpy as np
import numpy.typing as npt
import tqdm


def simulate_recording(
    spec: mj.MjSpec,
    recorded_actuator_positions: npt.NDArray[np.float64],
    *,
    sensors: Sequence[str],
    initial_keyframe: int = 0,
    camera_distance: float = 1.0,
    framerate: float = 30.0,
    video_path: str | None = None,
) -> npt.NDArray[np.float64]:
    model = spec.compile()
    data = mj.MjData(model)

    camera = mj.MjvCamera()
    mj.mjv_defaultFreeCamera(model, camera)
    camera.distance = camera_distance

    frames = []
    data = mj.MjData(model)
    mj.mj_resetDataKeyframe(model, data, initial_keyframe)

    simulated_sensor_data = []

    with mj.Renderer(model) as renderer:
        for actuators in tqdm.tqdm(recorded_actuator_positions, desc="simulating"):
            data.ctrl = actuators
            mj.mj_step(model, data)
            sensor_data = np.concatenate(
                [data.sensor(sensor).data for sensor in sensors],
            )
            simulated_sensor_data.append(sensor_data)
            if video_path is not None and len(frames) < data.time * framerate:
                camera.lookat = data.body("Nao").subtree_com
                renderer.update_scene(data, camera)
                pixels = renderer.render()
                frames.append(pixels)

    simulated_sensor_data = np.vstack(simulated_sensor_data)

    if video_path is not None:
        media.write_video(video_path, frames, fps=framerate)

    return simulated_sensor_data
