from _typeshed import Incomplete
from OpenGL.EGL import *

PFNEGLQUERYDEVICESEXTPROC: Incomplete
EGL_PLATFORM_DEVICE_EXT: int
PFNEGLGETPLATFORMDISPLAYEXTPROC: Incomplete
eglGetPlatformDisplayEXT: Incomplete

def eglQueryDevicesEXT(max_devices: int = 10): ...
