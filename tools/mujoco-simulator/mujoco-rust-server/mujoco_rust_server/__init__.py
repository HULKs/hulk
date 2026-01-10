import sys

from .mujoco_rust_server import *  # noqa: F403

sys.modules["mujoco_rust_server.ros2_types"] = mujoco_rust_server.ros2_types  # noqa: F405
sys.modules["mujoco_rust_server.booster_types"] = (
    mujoco_rust_server.booster_types  # noqa: F405
)
