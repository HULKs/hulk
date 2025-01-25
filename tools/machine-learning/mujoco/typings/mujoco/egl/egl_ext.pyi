from OpenGL.EGL import *
from _typeshed import Incomplete

PFNEGLQUERYDEVICESEXTPROC: Incomplete
EGL_PLATFORM_DEVICE_EXT: int
PFNEGLGETPLATFORMDISPLAYEXTPROC: Incomplete
eglGetPlatformDisplayEXT: Incomplete

def eglQueryDevicesEXT(max_devices: int = 10): ...
