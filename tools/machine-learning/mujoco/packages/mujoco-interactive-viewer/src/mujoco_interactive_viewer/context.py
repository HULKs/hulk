from mujoco_interactive_viewer.viewer import Viewer

_viewer: Viewer | None = None


def set_global_viewer(viewer: Viewer) -> None:
    global _viewer
    _viewer = viewer


def current_viewer() -> Viewer | None:
    return _viewer
