from pathlib import Path

import click
import openvino as ov
import torch
from nao_env.nao_standing import OFFSET_QPOS
from stable_baselines3 import PPO
from stable_baselines3.common.policies import BasePolicy
from torch import nn


class OnnxableSB3Policy(nn.Module):
    def __init__(self, policy: BasePolicy) -> None:
        super().__init__()
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

        return actions + torch.from_numpy(OFFSET_QPOS)


@click.command()
@click.option(
    "--load-policy",
    type=click.Path(exists=True),
    default=None,
    help="Load a policy from a file.",
)
def main(load_policy: str) -> None:
    path = Path(load_policy)
    name = path.parent.name
    model = PPO.load(load_policy)
    network = OnnxableSB3Policy(model.policy)
    observation_size = model.observation_space.shape

    Path("result").mkdir(exist_ok=True)

    observation = torch.zeros(1, *observation_size)

    print(observation.shape, network.forward(observation))

    with torch.inference_mode():
        torch.onnx.export(
            network,
            torch.randn(1, *observation_size),
            f"result/{name}-model.onnx",
            input_names=["input"],
            output_names=["output"],
            opset_version=17,
        )

    ov_model = ov.convert_model(f"result/{name}-model.onnx")
    ov.save_model(ov_model, f"result/{name}-policy-ov.xml")

    nn = ov.compile_model(ov_model)
    print(nn(observation))


if __name__ == "__main__":
    main()
