import ctypes
import enum
from _typeshed import Incomplete

CGLContextObj = ctypes.c_void_p
CGLPixelFormatObj = ctypes.c_void_p
GLint = ctypes.c_int

class CGLOpenGLProfile(enum.IntEnum):
    CGLOGLPVersion_Legacy = 4096
    CGLOGLPVersion_3_2_Core = 12800
    CGLOGLPVersion_GL3_Core = 12800
    CGLOGLPVersion_GL4_Core = 16640

class CGLPixelFormatAttribute(enum.IntEnum):
    CGLPFAAllRenderers = 1
    CGLPFATripleBuffer = 3
    CGLPFADoubleBuffer = 5
    CGLPFAColorSize = 8
    CGLPFAAlphaSize = 11
    CGLPFADepthSize = 12
    CGLPFAStencilSize = 13
    CGLPFAMinimumPolicy = 51
    CGLPFAMaximumPolicy = 52
    CGLPFASampleBuffers = 55
    CGLPFASample = 56
    CGLPFAColorFloat = 58
    CGLPFAMultisample = 59
    CGLPFASupersample = 60
    CGLPFASampleAlpha = 61
    CGLPFARendererID = 70
    CGLPFANoRecovery = 72
    CGLPFAAccelerated = 73
    CGLPFAClosestPolicy = 74
    CGLPFABackingStore = 76
    CGLPFABackingVolatile = 77
    CGLPFADisplayMask = 84
    CGLPFAAllowOfflineRenderers = 96
    CGLPFAAcceleratedCompute = 97
    CGLPFAOpenGLProfile = 99
    CGLPFASupportsAutomaticGraphicsSwitching = 101
    CGLPFAVirtualScreenCount = 128
    CGLPFAAuxBuffers = 7
    CGLPFAAccumSize = 14
    CGLPFAAuxDepthStencil = 57
    CGLPFAStereo = 6
    CGLPFAOffScreen = 53
    CGLPFAWindow = 80
    CGLPFACompliant = 83
    CGLPFAPBuffer = 90
    CGLPFARemotePBuffer = 91
    CGLPFASingleRenderer = 71
    CGLPFARobust = 75
    CGLPFAMPSafe = 78
    CGLPFAMultiScreen = 81
    CGLPFAFullScreen = 54

class CGLError(RuntimeError): ...

CGLChoosePixelFormat: Incomplete
CGLCreateContext: Incomplete
CGLLockContext: Incomplete
CGLReleaseContext: Incomplete
CGLReleasePixelFormat: Incomplete
CGLSetCurrentContext: Incomplete
CGLUnlockContext: Incomplete
