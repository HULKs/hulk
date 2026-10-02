# Konerl K1 motion-policy experiments

Konerl integrates custom K1 environments with MJLab. Use Python 3.13 and `uv`
from `tools/machine-learning/konerl`:

```bash
uv sync
uv run train --help
```

The project pins MJLab 1.3.0 and MuJoCo 3.7.0, uses MuJoCo Warp, and registers
`konerl.tasks.k1_standing` through the `mjlab.tasks` entry-point group in
`tools/machine-learning/konerl/pyproject.toml`. The checked-in task ID is
`Mjlab-Standing-K1`:

```bash
MUJOCO_GL=egl uv run train Mjlab-Standing-K1 --gpu-ids all
```

This uses the GPU training environment; configure compatible GPU drivers and
MJLab dependencies for the host. Inspect `src/konerl/tasks/k1_standing/env_cfg.py`
and `rl_cfg.py` for environment and PPO settings, and `src/konerl/k1_config.py`
for the model/joint configuration. Model assets are under `model`.

## Prototype limitations

`train.sh` currently requests `Mjlab-Velocity-Rough-K1`, which is not registered
by this package's checked-in task entry point. It is retained as an experiment
example, not a working recipe for the standing task.
`scripts/inference-example.py` also imports the absent
`konerl.tasks.k1_velocity_tracking` module, so it is not a runnable inference
workflow in this checkout. It would additionally require a compatible exported
ONNX policy and a desktop MuJoCo viewer.

Training here does not automatically export, select or deploy the five production
motion-inference policies in `etc/neural_networks`. Establish the policy's
observation/action contract and export/configuration workflow before using it
with `./simulator` or on a robot. See `tools/simulate/README.md` for testing the
existing production policies.
