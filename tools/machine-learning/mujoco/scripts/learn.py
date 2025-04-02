import os
from collections.abc import Callable
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import click
import gymnasium as gym
import nao_env
import torch
import wandb
from gymnasium.wrappers import TimeLimit
from nao_env.wrappers import (
    SingleEpisodeVideoRecorder,
)
from stable_baselines3.common.base_class import BaseAlgorithm
from stable_baselines3.common.callbacks import (
    EvalCallback,
)
from stable_baselines3.common.env_util import make_vec_env
from stable_baselines3.common.monitor import Monitor
from stable_baselines3.common.utils import get_device
from stable_baselines3.common.vec_env import SubprocVecEnv, VecEnv
from stable_baselines3.ppo import PPO


class UnexpectedAlgorithmError(ValueError):
    def __init__(self, algorithm: str) -> None:
        super().__init__(f"unexpected algorithm: {algorithm}")


@dataclass
class Hyperparameters:
    environment: str
    algorithm: str
    batch_size: int
    epochs: int
    nsteps: int
    steps_per_epoch: int
    throw_tomatoes: bool
    learning_rate: float
    entropy_coefficient: float
    max_grad_norm: float
    time_limit: int
    num_envs: int
    transfer_weights_from: str | None


def make_env(config: Hyperparameters) -> Callable[..., gym.Env]:
    environments = {
        "NaoStanding": nao_env.NaoStanding,
        "NaoStandup": nao_env.NaoStandup,
        "NaoWalking": nao_env.NaoWalking,
    }

    def _init(**kwargs: Any) -> gym.Env:
        env_cls = environments[config.environment]
        kwargs.update(
            {
                "throw_tomatoes": config.throw_tomatoes,
                "render_mode": "rgb_array",
            },
        )

        env = env_cls(**kwargs)
        return TimeLimit(env, max_episode_steps=config.time_limit)

    return _init


def build_train_env(config: Hyperparameters) -> VecEnv:
    return make_vec_env(
        env_id=make_env(config),
        n_envs=config.num_envs,
        vec_env_cls=SubprocVecEnv if config.num_envs > 1 else None,
        seed=42,
    )


def build_eval_env(run: Any, config: Hyperparameters) -> gym.Env:
    env = make_env(config)()
    env = Monitor(env)
    return SingleEpisodeVideoRecorder(
        env,
        f"videos/{run.name}",
    )


def setup_algorithm(
    run: Any,
    config: Hyperparameters,
    env: VecEnv,
) -> BaseAlgorithm:
    match config.algorithm:
        case "ppo":
            policy_kwargs = None
            if config.transfer_weights_from is not None:
                pretrained_ppo = PPO.load(config.transfer_weights_from)
                policy_kwargs = {"net_arch": pretrained_ppo.policy.net_arch}
            ppo = PPO(
                env=env,
                n_steps=config.nsteps,
                policy="MlpPolicy",
                batch_size=config.batch_size,
                learning_rate=config.learning_rate,
                max_grad_norm=config.max_grad_norm,
                ent_coef=config.entropy_coefficient,
                tensorboard_log=f"runs/{run.name}",
                policy_kwargs=policy_kwargs,
                verbose=1,
            )
            if config.transfer_weights_from is not None:
                ppo.policy.load_state_dict(pretrained_ppo.policy.state_dict())

            return ppo
        case _:
            raise UnexpectedAlgorithmError(config.algorithm)


@click.command()
@click.option(
    "--environment",
    type=click.Choice(["NaoStanding", "NaoStandup", "NaoWalking"]),
    default="NaoStanding",
)
@click.option("--algorithm", type=click.Choice(["ppo"]), default="ppo")
@click.option("--batch-size", type=click.INT, default=64)
@click.option("--epochs", type=click.INT, default=8000)
@click.option("--steps-per-epoch", type=click.INT, default=50_000)
@click.option("--nsteps", type=click.INT, default=2048)
@click.option("--throw-tomatoes", is_flag=True)
@click.option("--learning-rate", type=click.FLOAT, default=3e-4)
@click.option("--entropy-coefficient", type=click.FLOAT, default=1e-3)
@click.option("--max-grad-norm", type=click.FLOAT, default=0.5)
@click.option("--num-envs", type=click.INT, default=1)
@click.option("--time-limit", type=click.INT, default=4000)
@click.option("--wandb-project", type=click.STRING, default=None)
@click.option("--transfer-weights-from", type=click.STRING, default=None)
def main(
    *,
    environment: str,
    algorithm: str,
    batch_size: int,
    epochs: int,
    nsteps: int,
    throw_tomatoes: bool,
    steps_per_epoch: int,
    learning_rate: float,
    entropy_coefficient: float,
    max_grad_norm: float,
    num_envs: int,
    time_limit: int,
    wandb_project: str | None,
    transfer_weights_from: str | None,
) -> None:
    config = Hyperparameters(
        environment=environment,
        algorithm=algorithm,
        batch_size=batch_size,
        epochs=epochs,
        nsteps=nsteps,
        steps_per_epoch=steps_per_epoch,
        throw_tomatoes=throw_tomatoes,
        learning_rate=learning_rate,
        entropy_coefficient=entropy_coefficient,
        max_grad_norm=max_grad_norm,
        time_limit=time_limit,
        num_envs=num_envs,
        transfer_weights_from=transfer_weights_from,
    )
    run = wandb.init(
        project=wandb_project,
        config=asdict(config),
        monitor_gym=True,
        sync_tensorboard=True,
        mode="disabled" if wandb_project is None else "online",
    )

    train_env = build_train_env(config)
    eval_env = build_eval_env(run, config)
    rl_algorithm = setup_algorithm(run, config, train_env)

    rl_algorithm.learn(
        total_timesteps=config.epochs * config.steps_per_epoch,
        callback=EvalCallback(
            eval_env=eval_env,
            n_eval_episodes=1,
            eval_freq=config.steps_per_epoch,
            best_model_save_path=f"models/{run.name}",
        ),
        progress_bar=True,
    )


if __name__ == "__main__":
    os.environ["MUJOCO_GL"] = "egl"

    for device in range(torch.cuda.device_count()):
        print(f"Device {device}: {torch.cuda.get_device_name(device)}")

    NVIDIA_ICD_CONFIG_PATH = Path(
        "/usr/share/glvnd/egl_vendor.d/10_nvidia.json",
    )
    if not NVIDIA_ICD_CONFIG_PATH.exists() and get_device() != torch.device(
        "cpu",
    ):
        NVIDIA_ICD_CONFIG_PATH.write_text("""{
                                "file_format_version" : "1.0.0",
                                "ICD" : {
                                    "library_path" : "libEGL_nvidia.so.0"
                                }
                            }""")
    main()
