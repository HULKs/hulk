from dataclasses import dataclass
import os

import click
import gymnasium as gym
import torch
import wandb
from stable_baselines3 import DDPG, PPO, SAC
from stable_baselines3.common.monitor import Monitor
from stable_baselines3.common.utils import get_device, safe_mean
from stable_baselines3.common.vec_env import (
    VecVideoRecorder,
    SubprocVecEnv,
    VecMonitor,
    VecNormalize,
)
from wandb.integration.sb3 import WandbCallback
from gymnasium import logger
from moviepy.video.io.ImageSequenceClip import ImageSequenceClip

# Configure MuJoCo to use the EGL rendering backend (requires GPU)
os.environ["MUJOCO_GL"] = "egl"

gym.register(
    id="NaoStanding-v1",
    entry_point="nao_env:NaoStanding",
    max_episode_steps=2500,
)


@dataclass
class Config:
    policy_type: str
    total_timesteps: int
    env_name: str
    render_mode: str
    throw_tomatos: bool
    learning_rate: float
    n_steps: int
    max_grad_norm: float
    num_envs: int
    algorithm: str


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


class CustomPPO(PPO):
    def __init__(self, info_keywords: list[str], *args, **kwargs):
        self.info_keywords = info_keywords
        super().__init__(*args, **kwargs)

    def _dump_logs(self, iteration):
        if len(self.ep_info_buffer) > 0 and len(self.ep_info_buffer[0]) > 0:
            for keyword in self.info_keywords:
                self.logger.record(
                    f"rollout/ep_{keyword}_mean",
                    safe_mean([ep_info[keyword]
                              for ep_info in self.ep_info_buffer]),
                )
        super()._dump_logs(iteration)


class WorkingVecVideoRecorder(VecVideoRecorder):
    def _start_recording(self) -> None:
        self.video_name = f"{self.name_prefix}-step-{self.step_id}-to-step-{self.step_id + self.video_length}.mp4"
        self.vj= os.path.join(self.video_folder, self.video_name)
        super()._start_recording()

    def _stop_recording(self) -> None:
        super()._stop_recording()
        print("Logging to wandb...")
        wandb.log({"video": wandb.Video(
            self.video_path, format="mp4")}, step=self.step_id)


def video_trigger(episode_id: int) -> bool:
    return episode_id - 1000 % 20000 == 0


def make_env(config: Config):
    env = gym.make(config.env_name, throw_tomatos=config.throw_tomatos,
                   render_mode=config.render_mode)
    return env


def setup_env(run, config: Config):
    env = SubprocVecEnv([lambda: make_env(config)
                        for _ in range(config.num_envs)])
    env = VecNormalize(env, norm_obs=True, norm_reward=False)
    env = VecMonitor(
        env,
        info_keywords=("diff_ctrl", "head_ctrl"),
    )
    env = WorkingVecVideoRecorder(
        env,
        f"videos/{run.name}",
        record_video_trigger=video_trigger,
        video_length=600,
    )
    return env


def setup_algoritm(config: Config, env: gym.Env):
    match config.algorithm:
        case "ppo":
            return CustomPPO(
                info_keywords=("diff_ctrl", "head_ctrl"),
                n_steps=config.n_steps,
                policy=config.policy_type,
                env=env,
                learning_rate=config.learning_rate,
                max_grad_norm=config.max_grad_norm,
                device="cpu",
            )
        case "sac":
            return SAC(
                policy=config.policy_type,
                env=env,
                learning_rate=config.learning_rate,
                device="cpu",
            )
        case "ddpg":
            return DDPG(
                policy=config.policy_type,
                env=env,
                learning_rate=config.learning_rate,
                device="cpu",
            )
        case _:
            raise ValueError(f"Invalid algorithm: {config.algorithm}")


@click.command()
@click.option("--no-debug", is_flag=True)
@click.option("--num-envs", default=1, type=click.INT)
@click.option("--algorithm", default="ppo", type=click.Choice(["ppo", "sac", "ddpg"]))
def main(no_debug: bool, num_envs: int, algorithm: str):
    config = Config(
        policy_type="MlpPolicy",
        total_timesteps=100_000_000,
        env_name="NaoStanding-v1",
        render_mode="rgb_array",
        throw_tomatos=False,
        learning_rate=4e-4,
        n_steps=2048,
        max_grad_norm=0.05,
        num_envs=num_envs,
        algorithm=algorithm
    )

    run = wandb.init(
        project="nao_standing",
        config=config,
        monitor_gym=True,
        save_code=False,
        mode="online" if no_debug else "disabled",
    )

    env = setup_env(run, config)
    model = setup_algoritm(config, env)

    model.learn(
        total_timesteps=config.total_timesteps,
        callback=WandbCallback(
            model_save_path=f"models/{run.name}",
        ),
        progress_bar=True,
    )
    run.finish()


if __name__ == "__main__":
    main()
