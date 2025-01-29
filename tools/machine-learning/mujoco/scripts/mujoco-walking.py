import itertools
import time

import click
import mujoco
import numpy as np
from mujoco_interactive_viewer import Viewer
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
    _, _, _, _, infos = env.step(np.zeros(env.model.nu))
    env.reset()

    model = None
    if load_policy is not None:
        model = PPO.load(load_policy)

    dt = env.dt

    viewer = Viewer(env.model, env.data)
    rewards_figure = viewer.figure("rewards")
    rewards_figure.set_title("Rewards")
    rewards_figure.set_x_label("Step")
    for key in infos:
        rewards_figure.add_line(key)

    total_reward_figure = viewer.figure("total_reward")
    total_reward_figure.add_line("Total Reward")
    total_reward_figure.line_color("Total Reward", red=0.0, green=0.0, blue=1.0)
    total_reward_figure.set_x_label("Step")

    total_reward = 0.0
    action = np.zeros(env.model.nu)

    while viewer.is_alive:
        start_time = time.time()
        viewer.camera.lookat[:] = env.data.site("Robot").xpos
        observation, reward, _terminated, _truncated, infos = env.step(action)
        if model:
            action, _ = model.predict(observation, deterministic=True)

        total_reward += reward

        for key, value in infos.items():
            rewards_figure.push_data_to_line(key, value)

        total_reward_figure.push_data_to_line("Total Reward", total_reward)

        for x, y, z in itertools.product(*((range(-1, 2),) * 3)):
            viewer.add_marker(
                kind=mujoco.mjtGeom.mjGEOM_SPHERE,
                size=[0.02, 0, 0],
                position=0.1 * np.array([x, y, z]),
                rgba=0.5 * np.array([x + 1, y + 1, z + 1, 2]),
            )

        viewer.render()
        end_time = time.time()
        wait_time = max(0, dt - (end_time - start_time))
        time.sleep(wait_time)


if __name__ == "__main__":
    main()
