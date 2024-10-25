# Setup

Use [uv](https://docs.astral.sh/uv/).

- `uv sync`
- `source .venv/bin/activate`

## To build custom nao env

- In `lib/python3.10/site-packages/gymnasium/envs/mujoco/mujoco_rendering.py` change `self.data.solver_iter` to `self.data.solver_niter`
