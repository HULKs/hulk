import marimo

__generated_with = "0.23.6"
app = marimo.App(width="medium")


@app.cell
def _():
    import marimo as mo
    import workshop

    return mo, workshop


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
def _(mo):
    from pathlib import Path

    from mujoco import MjData, MjModel, Renderer, mj_step
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

    def advance_simulation(
        mj_model: MjModel, 
        mj_data: MjData, 
        dt: float,
    ) -> None:
        n_steps = int(dt / model.opt.timestep / 2)
        mj_step(mj_model, mj_data, nstep=n_steps)

    def update(_):
        advance_simulation(model, data, interval)
        renderer.update_scene(data, camera="overview_cam")
        rendered_pixels = renderer.render()
        viewer.update(rendered_pixels)


    refresh_timer = mo.ui.refresh(default_interval=interval, on_change=update)
    mo.vstack([refresh_timer, viewer])
    return


if __name__ == "__main__":
    app.run()
