import itertools
import time

import click
from mujoco_viewer import MujocoViewer
import numpy as np
from mujoco._render import MjrRect, mjr_figure
from mujoco._structs import MjvScene
from mujoco_interactive_viewer import InteractiveViewer
from nao_env import NaoWalking
from stable_baselines3 import PPO


def key_callback(key: int) -> None:
    print(f"Key pressed: {key}")


@click.command()
@click.option(
    "--throw-tomatoes", is_flag=True, help="Throw tomatoes at the Nao."
)
@click.option(
    "--load-policy",
    type=click.STRING,
    default=None,
    help="Load a policy from a file.",
)
def main(*, throw_tomatoes: bool, load_policy: str | None) -> None:
    env = NaoWalking(throw_tomatoes=throw_tomatoes)
    _, _, _, _, infos = env.step(np.zeros(env.model.nu))
    env.reset()

    model = None
    if load_policy is not None:
        model = PPO.load(load_policy)

    dt = env.dt

    viewer = InteractiveViewer(env.model, env.data)
    # viewer = MujocoViewer(env.model, env.data)
    # viewer.add_line_to_figure("Total Reward")
    # viewer.set_graph_name("Reward", 0)
    # viewer.set_x_label("Step", 0)
    # for key in infos:
    #     viewer.add_graph_line(key, line_data=0.0, fig_idx=0)
    # viewer.show_graph_legend(show_legend=True, fig_idx=0)
    # viewer.set_grid_divisions(x_div=10, y_div=5, x_axis_time=10.0, fig_idx=0)
    #
    # viewer.set_graph_name("Summed Reward", 1)
    # viewer.set_x_label("Step", 1)
    # viewer.add_graph_line("Total Reward", line_data=0.0, fig_idx=1)
    # viewer.show_graph_legend(show_legend=True, fig_idx=1)
    # viewer.set_grid_divisions(x_div=10, y_div=5, x_axis_time=10.0, fig_idx=1)

    total_reward = 0.0
    action = np.zeros(env.model.nu)
    # viewer.add_marker(pos=np.array([1, 1, 1]), label="text")

    while viewer.is_alive:
        start_time = time.time()
        # viewer.camera.lookat[:] = env.data.site("Robot").xpos
        observation, reward, _terminated, _truncated, infos = env.step(action)
        if model:
            action, _ = model.predict(observation, deterministic=True)

        total_reward += reward
        # for key, value in infos.items():
        #     viewer.update_graph_line(
        #         key,
        #         line_data=value,
        #         fig_idx=0,
        #     )
        # viewer.add_data_to_line(
        #     "Total Reward",
        #     line_data=total_reward,
        # )
        # breakpoint()
        # mujoco.mjr_figure(
        #     mujoco.MjrRect(100, 100, 100, 200),
        #     fig,
        #     mujoco.MjrContext(),
        # )

        # viewer.user_scn.ngeom = 0
        # i = 0
        # for x, y, z in itertools.product(*((range(-1, 2),) * 3)):
        #     mujoco.mjv_initGeom(
        #         viewer.user_scn.geoms[i],
        #         type=int(mujoco.mjtGeom.mjGeo),
        #         size=np.array([0.02, 1, 1]),
        #         pos=0.1 * np.array([x, y, z]),
        #         mat=np.eye(3).flatten(),
        #         rgba=0.5 * np.array([x + 1, y + 1, z + 1, 2]),
        #     )
        #     i += 1
        # viewer.user_scn.ngeom = i

        viewer.render()
        end_time = time.time()
        wait_time = max(0, dt - (end_time - start_time))
        time.sleep(wait_time)


if __name__ == "__main__":
    main()
