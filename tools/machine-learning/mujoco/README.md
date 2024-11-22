# Setup

Use [uv](https://docs.astral.sh/uv/).
After installing uv, run `uv sync` to install all dependencies.

- In your now installed python environment (e.g. per default in .venv/)
- In `lib/python3.10/site-packages/gymnasium/envs/mujoco/mujoco_rendering.py` change `self.data.solver_iter` to `self.data.solver_niter`

## Example usage

To train the standup task:

- `uv run standup.py`

## To build a custom NAO environment

Add a new `MujocoEnv` class in the `nao_env` folder and add it to the `__init__.py` file.
