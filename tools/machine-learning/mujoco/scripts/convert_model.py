from pathlib import Path

import click
import openvino as ov
import torch
from nao_env import nao_standing, nao_walking
from stable_baselines3 import PPO
from stable_baselines3.common.policies import ActorCriticPolicy
from torch import nn


class UndefinedObservationSpaceError(ValueError):
    def __init__(self) -> None:
        super().__init__("observation space must have a fixed size.")


class UndefinedActionSpaceError(ValueError):
    def __init__(self) -> None:
        super().__init__("action space must have a fixed size.")


class OnnxableSB3Policy(nn.Module):
    def __init__(self, policy: ActorCriticPolicy, offset: torch.Tensor) -> None:
        super().__init__()
        self.offset = offset
        self.policy = policy

    def unscale_action(self, scaled_action: torch.Tensor) -> torch.Tensor:
        low, high = (
            torch.from_numpy(self.policy.action_space.low),
            torch.from_numpy(self.policy.action_space.high),
        )
        return low + (0.5 * (scaled_action + 1.0) * (high - low))

    def clip_action(self, action: torch.Tensor) -> torch.Tensor:
        low, high = (
            torch.from_numpy(self.policy.action_space.low).to(torch.float32),
            torch.from_numpy(self.policy.action_space.high).to(torch.float32),
        )
        return torch.clamp(action, low, high)

    def forward(self, observation: torch.Tensor) -> torch.Tensor:
        actions = self.policy._predict(observation, deterministic=True)

        if self.policy.squash_output:
            actions = self.unscale_action(actions)
        else:
            actions = self.clip_action(actions)

        return actions + self.offset


@click.command()
@click.argument(
    "policy",
    type=click.Path(exists=True),
    help="The policy to convert to ONNX.",
)
@click.argument(
    "environment-type",
    type=click.Choice(["NaoStanding", "NaoStandup", "NaoWalking"]),
)
def main(policy: str, environment_type: str) -> None:
    path = Path(policy)
    name = path.parent.name
    model = PPO.load(policy)

    observation_size = model.observation_space.shape
    if observation_size is None:
        raise UndefinedObservationSpaceError()
    action_size = model.action_space.shape
    if action_size is None:
        raise UndefinedActionSpaceError()

    offset = {
        "NaoStanding": torch.from_numpy(nao_standing.OFFSET_QPOS),
        "NaoStandup": torch.zeros(action_size),
        "NaoWalking": torch.from_numpy(nao_walking.OFFSET_QPOS),
    }[environment_type]

    network = OnnxableSB3Policy(model.policy, offset)
    Path("result").mkdir(exist_ok=True)

    with torch.inference_mode():
        torch.onnx.export(
            network,
            (torch.randn(observation_size),),
            f"result/{name}-model.onnx",
            input_names=["input"],
            output_names=["output"],
            opset_version=17,
        )

    ov_model = ov.convert_model(f"result/{name}-model.onnx")
    ov.save_model(ov_model, f"result/{name}-policy-ov.xml")


if __name__ == "__main__":
    main()
