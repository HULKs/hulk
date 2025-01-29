import time

import click
import mujoco_viewer
import numpy as np
from nao_env import NaoWalking
from stable_baselines3 import PPO


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
    _, _, _, _, infos = env.step(np.zeros(env.action_space_size))
    env.reset()

    model = None
    if load_policy is not None:
        model = PPO.load(load_policy)

    dt = env.dt

    viewer = mujoco_viewer.MujocoViewer(
        env.model,
        env.data,
        panel_num=2,
    )
    viewer.set_graph_name("Reward", 0)
    viewer.set_x_label("Step", 0)
    for key in infos:
        viewer.add_graph_line(key, line_data=0.0, fig_idx=0)
    viewer.show_graph_legend(show_legend=True, fig_idx=0)
    viewer.set_grid_divisions(x_div=10, y_div=5, x_axis_time=10.0, fig_idx=0)

    viewer.set_graph_name("Summed Reward", 1)
    viewer.set_x_label("Step", 1)
    viewer.add_graph_line("Total Reward", line_data=0.0, fig_idx=1)
    viewer.show_graph_legend(show_legend=True, fig_idx=1)
    viewer.set_grid_divisions(x_div=10, y_div=5, x_axis_time=10.0, fig_idx=1)

    total_reward = 0.0
    action = np.zeros(env.action_space_size)

    while viewer.is_alive:
        start_time = time.time()
        viewer.cam.lookat[:] = env.data.site("Robot").xpos
        observation, reward, _terminated, _truncated, infos = env.step(action)
        if model:
            action, _ = model.predict(observation, deterministic=True)

        total_reward += reward
        for key, value in infos.items():
            viewer.update_graph_line(
                key,
                line_data=value,
                fig_idx=0,
            )
        viewer.update_graph_line(
            "Total Reward",
            line_data=total_reward,
            fig_idx=1,
        )
        viewer.render()
        end_time = time.time()
        wait_time = max(0, dt - (end_time - start_time))
        time.sleep(wait_time)


if __name__ == "__main__":
    main()
