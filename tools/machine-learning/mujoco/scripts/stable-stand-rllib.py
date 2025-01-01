from ray import tune
from ray.rllib.algorithms.ppo import PPOConfig

import wandb
import click
from tqdm.auto import trange
import gymnasium as gym
import os

import nao_env

# Configure MuJoCo to use the EGL rendering backend (requires GPU)
os.environ["MUJOCO_GL"] = "egl"


@click.command()
@click.option("--no-debug", is_flag=True)
def main(no_debug: bool):
    config = (
        PPOConfig()
        .resources(
            num_gpus=2,
        )
        .environment(nao_env.NaoStanding, env_config={"throw_tomatos": False})
        .env_runners(
            num_env_runners=8,
        )
        .evaluation(
            evaluation_num_env_runners=1,
        )
    )
    algorithm = config.build()

    run = wandb.init(
        project="nao_standing",
        save_code=False,
        config=algorithm.config,
        mode="online" if no_debug else "disabled",
    )

    for _ in trange(10000):
        result = algorithm.train()
        print(result.keys())
        wandb.log(result)

    print(algorithm.evaluate())


if __name__ == "__main__":
    main()
