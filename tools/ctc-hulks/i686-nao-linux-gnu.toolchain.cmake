set(CMAKE_SYSTEM_NAME Linux)
set(CMAKE_SYSTEM_VERSION 1)
set(CMAKE_SYSTEM_PROCESSOR x86)

set(GCC_COMPILER_VERSION "6.3.0" CACHE STRING "GCC Compiler version")

get_filename_component(BASEDIR "${CMAKE_CURRENT_LIST_FILE}" PATH)
get_filename_component(BASEDIR ${BASEDIR} ABSOLUTE)

set(NAO_LINUX_TOOLCHAIN ${BASEDIR}/x-tools/i686-nao-linux-gnu)
set(NAO_LINUX_SYSROOT	${NAO_LINUX_TOOLCHAIN}/i686-nao-linux-gnu/sysroot)

set(CMAKE_C_COMPILER       ${NAO_LINUX_TOOLCHAIN}/bin/i686-nao-linux-gnu-gcc     )
set(CMAKE_CXX_COMPILER     ${NAO_LINUX_TOOLCHAIN}/bin/i686-nao-linux-gnu-g++     )
set(CMAKE_Fortran_COMPILER ${NAO_LINUX_TOOLCHAIN}/bin/i686-nao-linux-gnu-gfortran)

set(CMAKE_CXX_FLAGS           ""  CACHE STRING "c++ flags")
set(CMAKE_C_FLAGS             ""  CACHE STRING "c flags")
set(CMAKE_SHARED_LINKER_FLAGS ""  CACHE STRING "shared linker flags")
set(CMAKE_MODULE_LINKER_FLAGS ""  CACHE STRING "module linker flags")
set(CMAKE_EXE_LINKER_FLAGS    ""  CACHE STRING "executable linker flags")

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -O2 -march=atom -mssse3 -mfpmath=sse -fomit-frame-pointer")
set(CMAKE_C_FLAGS   "${CMAKE_C_FLAGS} -O2 -march=atom -mssse3 -mfpmath=sse -fomit-frame-pointer")

set(CMAKE_SYSROOT ${NAO_LINUX_SYSROOT})
#set(CMAKE_FIND_ROOT_PATH ${CMAKE_FIND_ROOT_PATH} ${NAO_LINUX_SYSROOT})
set(CMAKE_FIND_ROOT_PATH ${NAO_LINUX_SYSROOT})

set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)

set(CMAKE_CROSSCOMPILING TRUE)

set(CMAKE_INSTALL_RPATH ${CMAKE_INSTALL_PREFIX}/lib)
