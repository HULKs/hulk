"""
Exports GLContext for MuJoCo Python bindings.
"""
from __future__ import annotations

import ctypes as ctypes
import os as os
import platform as platform

from mujoco.glfw import GLContext

__all__ = ['GLContext', 'ctypes', 'os', 'platform']
_MUJOCO_GL: str = ''
_SYSTEM: str = 'Linux'
_VALID_MUJOCO_GL: tuple = ('enable', 'enabled', 'on', 'true', '1', 'glfw', '', 'glx', 'egl', 'osmesa')
