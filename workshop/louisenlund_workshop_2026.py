import marimo

__generated_with = "0.23.6"
app = marimo.App(width="medium")


@app.cell
def _():
    import marimo as mo
    import io
    import contextlib
    import workshop
    import numpy as np
    import pickle
    from pathlib import Path

    return Path, mo, np, workshop


@app.cell
def _(mo, workshop):
    mo.md(f"""
    Version {workshop.TEST}
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    # Arm Animation
    """)
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    Der K1 Roboter hat in jedem Arm __3__ Gelenke:
        - Schulter heben
        - Schulter rotieren
        - Ellenbogen anwinkeln

    Alle Gelenke sind *Rotationsgelenke*, sie beschreiben eine Rotation auf einer zwei-dimensionalen Ebene.
    """)
    return


@app.cell
def _(Path, mo, thinking):
    from mujoco import MjData, MjModel, Renderer, mj_step, mj_forward
    from workshop import MujocoViewer
    import os

    os.environ["MUJOCO_GL"] = "glfw"

    model = MjModel.from_xml_path(str(Path("./K1/K1.xml").resolve()))
    model.vis.global_.offwidth = 1280
    model.vis.global_.offheight = 720
    data = MjData(model)
    renderer = Renderer(model, width=1280, height=720)
    viewer = mo.ui.anywidget(MujocoViewer())
    interval = 0.1

    animation_index = 0

    animation_frames = thinking.frames

    animation_length = len(animation_frames)

    arm_joints = [
       "ALeft_Shoulder_Pitch",
       "Left_Shoulder_Roll",
       "Left_Elbow_Pitch",
       "Left_Elbow_Yaw",
       "ARight_Shoulder_Pitch",
       "Right_Shoulder_Roll",
       "Right_Elbow_Pitch",
       "Right_Elbow_Yaw",
    ]

    def set_joint_positions(positions):
       for joint_name, position in zip(arm_joints, positions):
           joint = model.joint(joint_name)
           data.qpos[joint.qposadr[0]] = position

       data.qvel[:] = 0.0
       mj_forward(model, data)

    def advance_simulation(
        mj_model: MjModel, 
        mj_data: MjData, 
        dt: float,
    ) -> None:
        n_steps = int(dt / model.opt.timestep / 2)
        mj_step(mj_model, mj_data, nstep=n_steps)

    def update(_):
        global animation_index
        set_joint_positions(thinking.frames[animation_index])
        animation_index = (animation_index + 1) % animation_length
        advance_simulation(model, data, interval)
        renderer.update_scene(data, camera="overview_cam")
        rendered_pixels = renderer.render()
        viewer.update(rendered_pixels)



    refresh_timer = mo.ui.refresh(default_interval=interval, on_change=update)
    mo.vstack([refresh_timer, viewer])
    return


@app.cell(hide_code=True)
def _(Path, np):
    class Animation:
        def __init__(self, name: str) -> None:
            self.name = name
            self.fps = 50 # frequency in Hz
            self.frames = np.array([[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]])
            self.frame_times = np.array([0.0])
            self.key_frames = np.array([[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]])
            self.key_frame_times = np.array([0.0])
            self.durations = np.array([], dtype=float)
            self.joint_order = [
                "left_shoulder_pitch",
                "left_shoulder_roll",
                "left_shoulder_yaw",
                "left_elbow",
                "right_shoulder_pitch",
                "right_shoulder_roll",
                "right_shoulder_yaw",
                "right_elbow",
            ]

            self.joint_min = [-3.3, -1.74, -2.27, -2.44, -3.14, -1.57, -2.27, 0.0]
            self.joint_max = [1.22, 1.57, 2.27, 0.0, 1.22, 1.57, 0.7, 2.44]
            self.max_vel   = [6.0, 6.0, 7.0, 8.0, 6.0, 6.0, 7.0, 8.0]

        def neu(self, 
            dauer: float = 0.5, 
            positionen: np.ndarray = np.array([[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]])
        ) -> None:
            self.key_frames = np.vstack([self.key_frames, positionen])
            self.key_frame_times = np.append(self.key_frame_times, self.key_frame_times[-1] + dauer)
            self.durations = np.append(self.durations, dauer)

        def warte(
            self,
            dauer: float = 0.5,
        ) -> None:
            self.key_frames = np.vstack([self.key_frames, self.key_frames[-1]])
            self.key_frame_times = np.append(self.key_frame_times, self.key_frame_times[-1] + dauer)
            self.durations = np.append(self.durations, dauer)

        def compile(self) -> None:
            dt = 1.0 / self.fps

            joint_min = np.asarray(self.joint_min, dtype=float)
            joint_max = np.asarray(self.joint_max, dtype=float)
            max_vel = np.asarray(self.max_vel, dtype=float)

            key_frames = np.asarray(self.key_frames, dtype=float)

            if key_frames.ndim != 2:
                raise ValueError(f"key_frames must be 2D, got shape {key_frames.shape}")

            num_joints = key_frames.shape[1]

            if joint_min.shape != (num_joints,):
                raise ValueError(f"joint_min must have shape ({num_joints},), got {joint_min.shape}")

            if joint_max.shape != (num_joints,):
                raise ValueError(f"joint_max must have shape ({num_joints},), got {joint_max.shape}")

            if max_vel.shape != (num_joints,):
                raise ValueError(f"max_vel must have shape ({num_joints},), got {max_vel.shape}")

            if np.any(joint_min > joint_max):
                raise ValueError("joint_min must be <= joint_max for every joint")

            if np.any(max_vel < 0.0):
                raise ValueError("max_vel must be >= 0 for every joint")

            # Clamp keyframes first, so the generated trajectory is inside limits.
            key_frames = np.clip(key_frames, joint_min, joint_max)

            compiled_frames = []
            compiled_times = []

            t_global = 0.0

            # smootherstep derivative:
            # d/ds [10s^3 - 15s^4 + 6s^5] = 30s^2(1-s)^2
            # maximum is 1.875 at s = 0.5
            max_smootherstep_derivative = 1.875

            # Add the first frame once.
            compiled_frames.append(key_frames[0].copy())
            compiled_times.append(t_global)

            for i, requested_duration in enumerate(self.durations):
                start = key_frames[i]
                end = key_frames[i + 1]

                delta = end - start
                abs_delta = np.abs(delta)

                required_duration_per_joint = np.zeros(num_joints, dtype=float)

                moving = abs_delta > 0.0
                limited = max_vel > 0.0

                impossible = moving & ~limited
                if np.any(impossible):
                    bad_joints = np.flatnonzero(impossible)
                    raise ValueError(
                        "Some joints need to move but have max_vel <= 0. "
                        f"Joints: {bad_joints.tolist()}"
                    )

                valid = moving & limited
                required_duration_per_joint[valid] = (
                    abs_delta[valid] * max_smootherstep_derivative / max_vel[valid]
                )

                segment_duration = max(
                    float(requested_duration),
                    float(np.max(required_duration_per_joint)),
                )

                # Quantize to the frame grid. This guarantees actual_duration >= segment_duration.
                num_intervals = max(1, int(np.ceil(segment_duration * self.fps)))
                actual_duration = num_intervals * dt

                for step in range(1, num_intervals + 1):
                    s = step / num_intervals

                    blend = 10 * s**3 - 15 * s**4 + 6 * s**5
                    pose = start + delta * blend

                    # Numerical safety clamp.
                    pose = np.clip(pose, joint_min, joint_max)

                    t_global += dt
                    compiled_frames.append(pose.copy())
                    compiled_times.append(t_global)

            self.frames = np.asarray(compiled_frames, dtype=float)
            self.frame_times = np.asarray(compiled_times, dtype=float)

            print(f"Compiled into {len(self.frames)} frames")

        def export(self, datei_name: str | None = None) -> None:
            if datei_name is None:
                datei_name = self.name

            data = {}

            data["fps"] = self.fps
            data["positions"] = self.frames

            if not datei_name.endswith(".npy"):
                datei_name = f"animation/{datei_name}.npy"
            else:
                datei_name = f"animation/{datei_name}"

            file_path = Path(datei_name)
            file_path.parent.mkdir(parents=True, exist_ok=True)

            with open(file_path, "wb"):
                np.save(file=file_path, allow_pickle=True, arr=data)

        def load(self, datei_name: str) -> None:

            if not datei_name.endswith(".npy"):
                datei_name = f"animation/{datei_name}{".npy"}"
            else:
                datei_name = f"animation/{datei_name}"

            file_path = Path(datei_name)

            data = np.load(file=file_path, allow_pickle=True).item()

            self.fps = data["fps"]
            self.frames = data["positions"]

    return (Animation,)


@app.cell
def _(Animation, animation_code_box, compile_button, mo, np):
    mo.stop(not compile_button.value)

    namespace = {
        "np": np,
        "Animation": Animation,
        "thinking": Animation("thinking"),
    }

    exec(animation_code_box.value, namespace)

    thinking = namespace["thinking"]
    thinking.compile()
    return (thinking,)


@app.cell(hide_code=True)
def _(mo):
    initial_code = mo.ui.code_editor(
        """thinking = Animation("thinking")

    thinking.neu(
        dauer=0.3,
        positionen=np.array([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    )""",
        disabled=True,
    )

    animation_code_box = mo.ui.code_editor(
        """thinking.neu(dauer=0.3, positionen=np.array([0.0, -1.4, 0.0, -2.25, 0.0, 1.4, 0.0, 2.25]))

    for i in range(3):
        thinking.neu(dauer=0.1, positionen=np.array([0.0, -1.4, 0.0, -2.25, 0.0, 1.4, 0.0, 2.25]))
        thinking.neu(dauer=0.1, positionen=np.array([-2.0, -1.2, 0.0, -0.3, -2.0, 1.2, 0.0, 0.3]))""",
        language="python",
    )

    compile_button = mo.ui.run_button(label="Compile")
    export_button = mo.ui.run_button(label="Export")

    mo.vstack([
        initial_code,
        animation_code_box,
        mo.hstack([
            compile_button,
            export_button,
        ]),
    ])
    return animation_code_box, compile_button, export_button


@app.cell
def _(export_button, mo, thinking):
    mo.stop(not export_button.value)

    thinking.export()
    return


@app.cell
def _():
    return


if __name__ == "__main__":
    app.run()
