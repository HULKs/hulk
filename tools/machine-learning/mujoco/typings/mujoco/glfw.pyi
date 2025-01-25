"""
An OpenGL context created via GLFW.
"""
from __future__ import annotations
import glfw as glfw
import typing
__all__ = ['GLContext', 'glfw']
class GLContext:
    """
    An OpenGL context created via GLFW.
    """
    __firstlineno__: typing.ClassVar[int] = 20
    __static_attributes__: typing.ClassVar[tuple] = ('_context')
    def __del__(self):
        ...
    def __init__(self, max_width, max_height):
        ...
    def free(self):
        ...
    def make_current(self):
        ...
