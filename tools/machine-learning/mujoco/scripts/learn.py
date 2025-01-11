import os
from dataclasses import dataclass
from pathlib import Path

import click
import nao_env


@dataclass
class Hyperparameters:
    environment: str
    batch_size: int
    epochs: int
    steps_per_epoch: int
    throw_tomatos: bool
    learning_rate: float
    max_grad_norm: float
    num_envs: int


@click.command()
@click.option(
    "--environment",
    type=click.Choice(["NaoStanding", "NaoStandup"]),
    default="NaoStanding-v1",
)
@click.option("--batch-size", type=click.INT, default=64)
@click.option("--epochs", type=click.INT, default=1000)
@click.option("--steps-per-epoch", type=click.INT, default=100_000)
@click.option("--throw-tomatos", is_flag=True)
@click.option("--learning-rate", type=click.FLOAT, default=3e-4)
@click.option("--max-grad-norm", type=click.FLOAT, default=0.5)
@click.option("--num-envs", type=click.INT, default=1)
def main(
    **kwargs,
):
    config = Hyperparameters(**kwargs)
    print(config)


if __name__ == "__main__":
    os.environ["MUJOCO_GL"] = "egl"

    NVIDIA_ICD_CONFIG_PATH = Path(
        "/usr/share/glvnd/egl_vendor.d/10_nvidia.json"
    )
    if not NVIDIA_ICD_CONFIG_PATH.exists() and get_device() != torch.device(
        "cpu"
    ):
        NVIDIA_ICD_CONFIG_PATH.write_text("""{
                                "file_format_version" : "1.0.0",
                                "ICD" : {
                                    "library_path" : "libEGL_nvidia.so.0"
                                }
                            }""")
    nao_env.register()
    main()
