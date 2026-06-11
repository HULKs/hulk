import marimo

__generated_with = "0.23.6"
app = marimo.App(width="medium")


@app.cell(hide_code=True)
def _():
    import marimo as mo
    import workshop
    import numpy as np
    from pathlib import Path
    import io
    import contextlib

    return Path, contextlib, io, mo, np, workshop


@app.cell(hide_code=True)
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


@app.cell(hide_code=True)
def _(
    Path,
    animation,
    get_manual_joint_mode,
    get_manual_joint_positions,
    mo,
    set_manual_joint_mode,
    set_manual_joint_positions,
):
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

    animation_index = [0]
    animation_frames = animation.frames
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

    def render_from_positions(positions):
        set_joint_positions(positions)
        renderer.update_scene(data, camera="overview_cam")
        rendered_pixels = renderer.render()
        viewer.update(rendered_pixels)

    def advance_simulation(
        mj_model: MjModel, 
        mj_data: MjData, 
        dt: float,
    ) -> None:
        n_steps = int(dt / model.opt.timestep / 2)
        mj_step(mj_model, mj_data, nstep=n_steps)

    def update(_):
        set_joint_positions(animation_frames[animation_index[0]])
        animation_index[0] = (animation_index[0] + 1) % animation_length
        advance_simulation(model, data, interval)
        renderer.update_scene(data, camera="overview_cam")
        rendered_pixels = renderer.render()
        viewer.update(rendered_pixels)

    def _activate_joint_position_mode(_value: bool) -> None:
        if _value:
            set_manual_joint_mode(True)

    def _as_float_list(values) -> list[float]:
        if isinstance(values, tuple):
            values = list(values)

        if not isinstance(values, list):
            return [0.0] * len(arm_joints)

        num_joints = len(arm_joints)
        positions = [float(value) for value in values[:num_joints]]
        if len(positions) < num_joints:
            positions.extend([0.0] * (num_joints - len(positions)))

        return positions

    def _set_manual_joint_position(joint_index: int):
        def _set_joint_position(value: float) -> None:
            positions = _as_float_list(get_manual_joint_positions())
            positions[joint_index] = float(value)
            set_manual_joint_positions(positions)

        return _set_joint_position

    manual_mode = bool(get_manual_joint_mode())
    manual_joint_positions = _as_float_list(get_manual_joint_positions())

    if manual_mode:
        arm_slider_controls = []
        for joint_index, (joint_name, joint_min, joint_max) in enumerate(
            zip(animation.joint_order, animation.joint_min, animation.joint_max)
        ):
            arm_slider_controls.append(
                mo.ui.slider(
                    value=manual_joint_positions[joint_index],
                    start=joint_min,
                    stop=joint_max,
                    step=0.01,
                    label=joint_name,
                    on_change=_set_manual_joint_position(joint_index),
                )
            )

        render_from_positions(manual_joint_positions)

        simulator_ui = mo.vstack(
            [
                viewer,
                mo.vstack(arm_slider_controls),
                mo.md(str(manual_joint_positions)),
            ]
        )
    else:
        refresh_timer = mo.ui.refresh(default_interval=interval, on_change=update)
        manual_position_button = mo.ui.run_button(
            label="Set Joint Positions",
            on_change=_activate_joint_position_mode,
        )
        simulator_ui = mo.vstack([refresh_timer, viewer, manual_position_button])

    simulator_ui
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
            self.joint_max = [1.22, 1.57, 2.27, 0.0, 1.22, 1.57, 2.27, 2.44]
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

                num_intervals = max(1, int(np.ceil(segment_duration * self.fps)))

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

            data = {
                "fps": self.fps,
                "positions": self.frames,
            }

            if not datei_name.endswith(".npy"):
                datei_name = f"animation/{datei_name}.npy"
            else:
                datei_name = f"animation/{datei_name}"

            file_path = Path(datei_name)
            file_path.parent.mkdir(parents=True, exist_ok=True)

            np.save(file=file_path, allow_pickle=True, arr=data)

        def load(self, datei_name: str) -> None:
            if not datei_name.endswith(".npy"):
                datei_name = f"animation/{datei_name}.npy"
            else:
                datei_name = f"animation/{datei_name}"

            file_path = Path(datei_name)

            data = np.load(file=file_path, allow_pickle=True).item()

            self.fps = data["fps"]
            self.frames = data["positions"]

    return (Animation,)


@app.function(hide_code=True)
def make_identifier(text: str) -> str:
    import keyword

    cleaned = "".join(
        character if character.isalnum() or character == "_" else "_"
        for character in text.strip()
    )
    cleaned = cleaned.strip("_") or "animation"

    if cleaned[0].isdigit():
        cleaned = f"animation_{cleaned}"

    if not cleaned.isidentifier() or keyword.iskeyword(cleaned):
        cleaned = "animation"

    return cleaned


@app.function(hide_code=True)
def default_animation_code(animation_name: str) -> str:
    return """animation.neu(dauer=0.3, positionen=np.array([0.0, -1.4, 0.0, -2.25, 0.0, 1.4, 0.0, 2.25]))

for i in range(3):
    animation.neu(dauer=0.1, positionen=np.array([0.0, -1.4, 0.0, -2.25, 0.0, 1.4, 0.0, 2.25]))
    animation.neu(dauer=0.1, positionen=np.array([-2.0, -1.2, 0.0, -0.3, -2.0, 1.2, 0.0, 0.3]))"""


@app.function(hide_code=True)
def normalize_animation_code(code: str) -> str:
    import re

    return re.sub(
        r"\b[A-Za-z_]\w*\s*\.\s*(neu|warte)\s*\(",
        r"animation.\1(",
        code,
    )


@app.cell(hide_code=True)
def _(mo):
    get_active_animation_name, set_active_animation_name = mo.state(None)
    get_animation_projects, set_animation_projects = mo.state({})
    get_compile_request, set_compile_request = mo.state(
        {
            "animation_name": None,
            "animation_code": None,
        }
    )
    get_manual_joint_mode, set_manual_joint_mode = mo.state(
        False,
        allow_self_loops=True,
    )
    get_manual_joint_positions, set_manual_joint_positions = mo.state(
        [0.0] * 8,
        allow_self_loops=True,
    )
    return (
        get_active_animation_name,
        get_animation_projects,
        get_compile_request,
        get_manual_joint_mode,
        get_manual_joint_positions,
        set_active_animation_name,
        set_animation_projects,
        set_compile_request,
        set_manual_joint_mode,
        set_manual_joint_positions,
    )


@app.cell(hide_code=True)
def _(Animation, get_compile_request, mo, np):
    _compile_request = get_compile_request()
    _animation_name = _compile_request["animation_name"]
    mo.stop(_animation_name is None)
    _animation_code = _compile_request["animation_code"]
    if _animation_code is not None:
        _animation_code = normalize_animation_code(_animation_code)

    _default_animation = Animation(_animation_name)
    _namespace = {
        "np": np,
        "Animation": Animation,
        "animation": _default_animation,
        "animation_name": _animation_name,
    }

    _keys_before_exec = set(_namespace)
    if _animation_code is not None:
        exec(_animation_code, _namespace)

    _new_animations = [
        value
        for key, value in _namespace.items()
        if key not in _keys_before_exec and isinstance(value, Animation)
    ]

    if len(_new_animations) == 1:
        animation = _new_animations[0]
    elif isinstance(_namespace.get("animation"), Animation):
        animation = _namespace["animation"]
    else:
        raise ValueError(
            "No animation found. Use the current animation variable, or create one "
            "Animation object, for example `wave = Animation('wave')`."
        )

    animation.compile()
    return (animation,)


@app.cell(hide_code=True)
def _(
    get_active_animation_name,
    get_animation_projects,
    mo,
    set_active_animation_name,
    set_animation_projects,
    set_compile_request,
):
    _projects = get_animation_projects()
    _active_animation_name = get_active_animation_name()

    animation_name_input = mo.ui.text(
        value="",
        label="New animation name",
        placeholder="thinking",
    )

    def _activate_project(animation_name: str, projects: dict | None = None) -> None:
        if projects is None:
            projects = get_animation_projects()

        project = projects[animation_name]
        set_active_animation_name(animation_name)
        set_compile_request(
            {
                "animation_name": animation_name,
                "animation_code": project["code"],
            }
        )

    def _submit_name(_value):
        if _value:
            _animation_name = animation_name_input.value.strip()
            if not _animation_name:
                return

            _projects = dict(get_animation_projects())
            if _animation_name not in _projects:
                _projects[_animation_name] = {
                    "name": _animation_name,
                    "code": default_animation_code(_animation_name),
                }
                set_animation_projects(_projects)

            _activate_project(_animation_name, _projects)

    def _select_project(value):
        if value:
            _activate_project(value)

    submit_name_button = mo.ui.run_button(label="Create", on_change=_submit_name)

    controls = [mo.hstack([animation_name_input, submit_name_button])]

    if _projects:
        _project_names = sorted(_projects)
        _dropdown_value = (
            _active_animation_name
            if _active_animation_name in _projects
            else _project_names[0]
        )
        project_dropdown = mo.ui.dropdown(
            options=_project_names,
            value=_dropdown_value,
            label="Projects",
            on_change=_select_project,
        )
        controls.insert(0, project_dropdown)

    mo.vstack(controls)
    return


@app.cell(hide_code=True)
def _(
    get_active_animation_name,
    get_animation_projects,
    mo,
    set_active_animation_name,
    set_animation_projects,
    set_compile_request,
    set_manual_joint_mode,
):
    _animation_name = get_active_animation_name()
    _projects = get_animation_projects()
    mo.stop(_animation_name is None or _animation_name not in _projects)

    _animation_name_literal = repr(_animation_name)
    _animation_code = normalize_animation_code(_projects[_animation_name]["code"])

    initial_code = mo.ui.code_editor(
        f"""animation = Animation({_animation_name_literal})

    animation.neu(
    dauer=0.3,
    positionen=np.array([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    )""",
        disabled=True,
    )

    animation_code_box = mo.ui.code_editor(
        _animation_code,
        language="python",
    )

    rename_name_input = mo.ui.text(
        value=_animation_name,
        label="Rename animation",
        placeholder=_animation_name,
    )

    def _save_current_code() -> str:
        _projects = dict(get_animation_projects())
        _project = dict(_projects[_animation_name])
        _project["code"] = normalize_animation_code(animation_code_box.value)
        _projects[_animation_name] = _project
        set_animation_projects(_projects)
        return _project["code"]

    def _rename(_value):
        if _value:
            _new_animation_name = rename_name_input.value.strip()
            if not _new_animation_name or _new_animation_name == _animation_name:
                return

            _projects = dict(get_animation_projects())
            if _new_animation_name in _projects:
                return

            _project = dict(_projects.pop(_animation_name))
            _project["name"] = _new_animation_name
            _project["code"] = normalize_animation_code(animation_code_box.value)
            _projects[_new_animation_name] = _project

            set_animation_projects(_projects)
            set_active_animation_name(_new_animation_name)
            set_compile_request(
                {
                    "animation_name": _new_animation_name,
                    "animation_code": _project["code"],
                }
            )

    def _save(_value):
        if _value:
            _save_current_code()

    def _compile(_value):
        if _value:
            set_manual_joint_mode(False)
            _animation_code = _save_current_code()
            set_compile_request(
                {
                    "animation_name": _animation_name,
                    "animation_code": _animation_code,
                }
            )

    rename_button = mo.ui.run_button(label="Rename", on_change=_rename)
    save_button = mo.ui.run_button(label="Save", on_change=_save)
    compile_button = mo.ui.run_button(label="Compile", on_change=_compile)
    export_button = mo.ui.run_button(label="Export")

    mo.vstack([
        mo.md(f"### Animation: `{_animation_name}`"),
        mo.hstack([rename_name_input, rename_button]),
        initial_code,
        animation_code_box,
        mo.hstack([
            save_button,
            compile_button,
            export_button,
        ]),
    ])
    return (export_button,)


@app.cell(hide_code=True)
def _(animation, export_button, mo):
    mo.stop(not export_button.value)

    animation.export()
    return


@app.cell(hide_code=True)
def _(mo):
    mo.md(r"""
    # Ball-Erkennung

    In diesem Teil arbeitet ihr mit Bildern als Pixel-Matrix. Jedes Pixel hat drei Farbwerte:

    - `R`: Rot
    - `G`: Grün
    - `B`: Blau

    Jeder farbwert geht von 0 bis 255.

    Ein roter Ball besteht also aus vielen Pixeln, bei denen der rote Wert deutlich
    größer ist als Grün und Blau.

    ## Aufgabe 1: Einen roten Ball finden

    Ziel: Findet den roten Ball im Bild und gebt die Bildkoordinaten seines Mittelpunkts aus.

    Eure Schritte:

    1. Lest für jedes Pixel die Werte `r`, `g` und `b` aus.
    2. Entscheidet, ob das Pixel rot genug ist.
    3. Speichert die Koordinaten aller roten Pixel.
    4. Berechnet daraus den Mittelpunkt des Balls.
    5. Zeichnet den Mittelpunkt in das Bild.
    """)
    return


@app.cell
def _(mo):
    mo.image(src="src/Ball.png", alt="Roter Ball")
    return


@app.cell
def _(mo):
    editor = mo.ui.code_editor("""from PIL import Image, ImageDraw
    import numpy as np


    def find_ball() -> Image.Image:
        img_raw = Image.open("src/Ball.png")
        img_array = np.array(img_raw)

        height, width, _ = img_array.shape

        red_pixels = []

        for y in range(height):
            for x in range(width):
                r = img_array[y, x, 0]
                g = img_array[y, x, 1]
                b = img_array[y, x, 2]

                # TODO 1: Findet eine gute Bedingung für rote Pixel.
                # Tipp: Rot sollte groß sein, Grün und Blau eher klein.
                is_red = False

                if is_red:
                    red_pixels.append((x, y))

        draw = ImageDraw.Draw(img_raw)

        if red_pixels:
            # TODO 2: Berechnet den Mittelpunkt aller roten Pixel.
            # Tipp: Der Mittelpunkt ist der Durchschnitt aller x- und y-Werte.
            center_x = 0
            center_y = 0
            print(f"Ball gefunden bei: {center_x}, {center_y}")

            radius = 20
            left_up = (center_x - radius, center_y - radius)
            right_down = (center_x + radius, center_y + radius)
            draw.ellipse([left_up, right_down], fill="blue", outline="blue")
        else:
            print("Kein Ball gefunden")

        return img_raw

    result_image = find_ball()""")
    editor
    return (editor,)


@app.cell
def _(contextlib, editor, io, mo):
    task_1_namespace = {}
    task_1_output_buffer = io.StringIO()
    with contextlib.redirect_stdout(task_1_output_buffer):
        try:
            exec(editor.value, task_1_namespace)
        except Exception as e:
            print(f"Execution Error: {e}")

    task_1_outputs = [mo.md(rf"""
    **Ausgabe**:
    ```markdown
    {task_1_output_buffer.getvalue()}
    ```
    """)]

    task_1_result_image = task_1_namespace.get("result_image")
    if task_1_result_image is not None:
        task_1_image_buffer = io.BytesIO()
        task_1_result_image.save(task_1_image_buffer, format="PNG")
        task_1_outputs.append(
            mo.image(src=task_1_image_buffer.getvalue(), alt="Ergebnisbild")
        )

    mo.vstack(task_1_outputs)
    return


if __name__ == "__main__":
    app.run()
