from mjlab.managers.termination_manager import TerminationTermCfg
from mjlab.managers.scene_entity_config import SceneEntityCfg
from mjlab.tasks.velocity import mdp
import torch

class DelayedBaseHeight:
    """Stateful termination term to delay termination by N steps."""
    def __init__(self, delay_steps: int):
        self.delay_steps = delay_steps
        self._counter: torch.Tensor | None = None
        self._termination_triggered: torch.Tensor | None = None

    def reset(self, env_ids: torch.Tensor | slice | None = None) -> None:
        """Called automatically by TerminationManager on environment resets."""
        if self._counter is not None:
            if env_ids is None:
                env_ids = slice(None)
            self._counter[env_ids] = 0
        if self._termination_triggered is not None:
            if env_ids is None:
                env_ids = slice(None)
            self._termination_triggered[env_ids] = False

    def __call__(
        self, 
        env, 
        minimum_height: float, 
        asset_cfg
    ) -> torch.Tensor:
        """Evaluates the condition and applies the delay."""
        if self._counter is None:
            self._counter = torch.zeros(env.num_envs, dtype=torch.long, device=env.device)
        
        if self._termination_triggered is None:
            self._termination_triggered = torch.zeros(env.num_envs, dtype=torch.long, device=env.device).bool()
        
        asset = env.scene[asset_cfg.name]
        condition_met = asset.data.root_link_pos_w[:, 2] < minimum_height

        self._termination_triggered.logical_or_(condition_met)
        self._counter = torch.where(
            self._termination_triggered, 
            self._counter + 1, 
            self._counter
        )

        return self._counter >= self.delay_steps

def make_termination_cfg() -> dict[str, TerminationTermCfg]:
    delayed_bad_base_height = DelayedBaseHeight(delay_steps=5)

    return {
        "time_out": TerminationTermCfg(func=mdp.time_out, time_out=True),
        "bad_base_height": TerminationTermCfg(
            func=delayed_bad_base_height, 
            params={
                "minimum_height": 0.3,
                "asset_cfg": SceneEntityCfg("robot")
            },
        ),
    }