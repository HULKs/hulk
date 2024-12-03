# Setup

Make sure `glfw` is installed on your machine.

For python use [uv](https://docs.astral.sh/uv/).
After installing uv, run `uv sync` to install all python dependencies or directly execute an example from below.

## Example usage

To view the model:

- `uv run interactive_viewer.py`

To train the standup task:

- `uv run standup.py`

## To build a custom NAO environment

Add a new `MujocoEnv` class in the `nao_env` folder and add it to the `__init__.py` file.
