import os
from dataclasses import dataclass

import click
import gymnasium as gym
import torch
import wandb
from gymnasium.wrappers import RecordVideo
from stable_baselines3 import DDPG, PPO, SAC
from stable_baselines3.common.callbacks import (
    CallbackList,
    CheckpointCallback,
    EvalCallback,
    ProgressBarCallback,
)
from stable_baselines3.common.monitor import Monitor
from stable_baselines3.common.utils import get_device
from stable_baselines3.common.vec_env import (
    SubprocVecEnv,
    VecVideoRecorder,
)
from wandb.integration.sb3 import WandbCallback

# Configure MuJoCo to use the EGL rendering backend (requires GPU)
os.environ["MUJOCO_GL"] = "egl"
os.environ["CUDA_VISIBLE_DEVICES"] = "0"

gym.register(
    id="NaoWalking-v1",
    entry_point="nao_env:NaoWalking",
    max_episode_steps=2500,
)


@dataclass
class Config:
    policy_type: str
    batch_size: int
    epochs: int
    steps_per_epoch: int
    env_name: str
    render_mode: str
    throw_tomatos: bool
    learning_rate: float
    n_steps: int
    max_grad_norm: float
    num_envs: int
    algorithm: str


class WandbLogVideoRecorder(RecordVideo):
    def stop_recording(self):
        path = os.path.join(self.video_folder, f"{self._video_name}.mp4")
        super().stop_recording()

        print("Logging to wandb...")
        wandb.log({"video": wandb.Video(path, format="mp4")}, step=self.step_id)


if get_device() != torch.device("cpu"):
    NVIDIA_ICD_CONFIG_PATH = "/usr/share/glvnd/egl_vendor.d/10_nvidia.json"
    if not os.path.exists(NVIDIA_ICD_CONFIG_PATH):
        with open(NVIDIA_ICD_CONFIG_PATH, "w") as f:
            _ = f.write("""{
                                "file_format_version" : "1.0.0",
                                "ICD" : {
                                    "library_path" : "libEGL_nvidia.so.0"
                                }
                            }""")


class WorkingVecVideoRecorder(VecVideoRecorder):
    def _start_recording(self) -> None:
        self.video_name = f"{self.name_prefix}-step-{self.step_id}-to-step-{self.step_id + self.video_length}.mp4"
        self.video_path = os.path.join(self.video_folder, self.video_name)
        super()._start_recording()

    def _stop_recording(self) -> None:
        super()._stop_recording()
        if wandb.run is not None:
            print("Logging to wandb...")
            wandb.log(
                {"video": wandb.Video(self.video_path, format="mp4")},
                step=self.step_id,
            )


def make_env(config: Config):
    env = gym.make(
        config.env_name,
        throw_tomatos=config.throw_tomatos,
        render_mode=config.render_mode,
    )
    env = Monitor(env)
    return env


def setup_train_env(run, config: Config):
    env = SubprocVecEnv(
        [lambda: make_env(config) for _ in range(config.num_envs)]
    )
    # env = VecMonitor(env)
    return env


def setup_eval_env(run, config: Config):
    env = gym.make(
        config.env_name,
        throw_tomatos=config.throw_tomatos,
        render_mode=config.render_mode,
    )
    env = Monitor(env)
    env = WandbLogVideoRecorder(
        env,
        f"videos/{run.name}",
        episode_trigger=lambda _: True,
        disable_logger=True,
    )
    return env


def setup_algorithm(run, config: Config, env: gym.Env):
    match config.algorithm:
        case "ppo":
            return PPO(
                n_steps=config.n_steps,
                policy=config.policy_type,
                batch_size=config.batch_size,
                env=env,
                learning_rate=config.learning_rate,
                max_grad_norm=config.max_grad_norm,
                verbose=1,
                tensorboard_log=f"runs/{run.name}",
            )
        case "sac":
            return SAC(
                policy=config.policy_type,
                env=env,
                learning_rate=config.learning_rate,
            )
        case "ddpg":
            return DDPG(
                policy=config.policy_type,
                env=env,
                learning_rate=config.learning_rate,
            )
        case _:
            raise ValueError(f"Invalid algorithm: {config.algorithm}")


@click.command()
@click.option("--no-debug", is_flag=True)
@click.option("--num-envs", default=1, type=click.INT)
@click.option(
    "--algorithm", default="ppo", type=click.Choice(["ppo", "sac", "ddpg"])
)
def main(no_debug: bool, num_envs: int, algorithm: str):
    config = Config(
        policy_type="MlpPolicy",
        batch_size=128,
        epochs=1000,
        steps_per_epoch=10000,
        env_name="NaoWalking-v1",
        render_mode="rgb_array",
        throw_tomatos=False,
        learning_rate=1e-4,  # 3e-4
        n_steps=2048,
        max_grad_norm=0.1,  # 0.5
        num_envs=num_envs,
        algorithm=algorithm,
    )

    run = wandb.init(
        project="nao_walking",
        config=config,
        monitor_gym=True,
        sync_tensorboard=True,
        save_code=False,
        mode="online" if no_debug else "disabled",
    )

    train_env = setup_train_env(run, config)
    eval_env = setup_eval_env(run, config)
    model = setup_algorithm(run, config, train_env)

    model.learn(
        total_timesteps=config.steps_per_epoch * config.epochs,
        callback=CallbackList(
            [
                WandbCallback(
                    model_save_path=f"models/{run.name}",
                    verbose=2,
                ),
                EvalCallback(
                    eval_env,
                    n_eval_episodes=1,
                    callback_on_new_best=CheckpointCallback(
                        config.steps_per_epoch, f"models/{run.name}"
                    ),
                    eval_freq=config.steps_per_epoch,
                ),
                ProgressBarCallback(),
            ]
        ),
    )
    run.finish()


if __name__ == "__main__":
    main()
