## Copyright (C) 2011, 2012 Aldebaran Robotics

# CMake toolchain file to cross compile on @arch@

# Altough the code look complex, all it does is:
# * Force C and CXX compiler to make sure we use gcc from cross.

##
# Utility macros
#espace space (this allow ctc path with space)
macro(set_escaped name)
  string(REPLACE " " "\\ " ${name} ${ARGN})
endmacro()
#double!
macro(set_escaped2 name)
  string(REPLACE " " "\\\\ " ${name} ${ARGN})
endmacro()

get_filename_component(_ROOT_DIR ${CMAKE_CURRENT_LIST_FILE} PATH)
get_filename_component(_TC_DIR ${_ROOT_DIR} PATH)

# This is filled in by the 4-install script.
set(CTC_SYSROOT_VERSION "INSERT_VERSION_HERE")

set(CTC_ROOTDIR "${_ROOT_DIR}/root")
set(CTC_SYSROOT "${CTC_ROOTDIR}/i686-nao-linux-gnu/sysroot")
set(CTC_LIBROOT "${CTC_ROOTDIR}/libroot")

##
# Define the target...
# But first, force cross-compilation, even if we are compiling
# from linux-x86 to linux-x86 ...
set(CMAKE_CROSSCOMPILING   ON)
# Then, define the target system
set(CMAKE_SYSTEM_NAME      "Linux")
set(CMAKE_SYSTEM_PROCESSOR "i686")
set(CMAKE_EXECUTABLE_FORMAT "ELF")

##
# Probe the build/host system...
set(_BUILD_EXT "")

# root of the cross compiled filesystem
#should be set but we do find_path in each module outside this folder !!!!
if(NOT CMAKE_FIND_ROOT_PATH)
  set(CMAKE_FIND_ROOT_PATH)
endif()
list(INSERT CMAKE_FIND_ROOT_PATH 0 "${CTC_ROOTDIR}" "${CTC_LIBROOT}")
# search for programs in the build host directories
set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM BOTH)
# for libraries and headers in the target directories
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)

set(CMAKE_FIND_ROOT_PATH ${CMAKE_FIND_ROOT_PATH} CACHE INTERNAL "" FORCE)

set(CMAKE_C_COMPILER   "${CTC_ROOTDIR}/bin/gclang${_BUILD_EXT}" Clang)
set(CMAKE_CXX_COMPILER "${CTC_ROOTDIR}/bin/gclang++${_BUILD_EXT}" Clang)
set(GCC_COMPILER "${CTC_ROOTDIR}/bin/i686-nao-linux-gnu-g++" CACHE FILEPATH "" FORCE)

set(CMAKE_LINKER  "${CTC_ROOTDIR}/bin/i686-nao-linux-gnu-ld${_BUILD_EXT}"      CACHE FILEPATH "" FORCE)
set(CMAKE_AR      "${CTC_ROOTDIR}/bin/i686-nao-linux-gnu-ar${_BUILD_EXT}"      CACHE FILEPATH "" FORCE)
set(CMAKE_RANLIB  "${CTC_ROOTDIR}/bin/i686-nao-linux-gnu-ranlib${_BUILD_EXT}"  CACHE FILEPATH "" FORCE)
set(CMAKE_NM      "${CTC_ROOTDIR}/bin/i686-nao-linux-gnu-nm${_BUILD_EXT}"      CACHE FILEPATH "" FORCE)
set(CMAKE_OBJCOPY "${CTC_ROOTDIR}/bin/i686-nao-linux-gnu-objcopy${_BUILD_EXT}" CACHE FILEPATH "" FORCE)
set(CMAKE_OBJDUMP "${CTC_ROOTDIR}/bin/i686-nao-linux-gnu-objdump${_BUILD_EXT}" CACHE FILEPATH "" FORCE)
set(CMAKE_STRIP   "${CTC_ROOTDIR}/bin/i686-nao-linux-gnu-strip${_BUILD_EXT}"   CACHE FILEPATH "" FORCE)

# If ccache is found, just use it:)
find_program(CCACHE "ccache")
if (CCACHE)
  message( STATUS "Using ccache")
endif(CCACHE)

if (CCACHE AND NOT FORCE_NO_CCACHE)
  set(CMAKE_C_COMPILER                 "${CCACHE}" CACHE FILEPATH "" FORCE)
  set(CMAKE_CXX_COMPILER               "${CCACHE}" CACHE FILEPATH "" FORCE)
  set_escaped2(CMAKE_C_COMPILER_ARG1   "${CTC_ROOTDIR}/bin/gclang${_BUILD_EXT}")
  set_escaped2(CMAKE_CXX_COMPILER_ARG1 "${CTC_ROOTDIR}/bin/gclang++${_BUILD_EXT}")
else(CCACHE AND NOT FORCE_NO_CCACHE)
  set_escaped(CMAKE_C_COMPILER         "${CTC_ROOTDIR}/bin/gclang${_BUILD_EXT}")
  set_escaped(CMAKE_CXX_COMPILER       "${CTC_ROOTDIR}/bin/gclang++${_BUILD_EXT}")
endif(CCACHE AND NOT FORCE_NO_CCACHE)

##
# Set target flags
set_escaped(CTC_ROOTDIR ${CTC_ROOTDIR})
set_escaped(CTC_SYSROOT ${CTC_SYSROOT})

# Show no mercy and erase previously set CMAKE_*_FLAGS
# (prevent unecessary re-compilations)
set(CMAKE_C_FLAGS "" CACHE STRING "" FORCE)
set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -target i686-nao-linux-gnu" CACHE STRING "" FORCE)
set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} --gcc-toolchain=${CTC_ROOTDIR}" CACHE STRING "" FORCE)
set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} --sysroot=${CTC_SYSROOT}" CACHE STRING "" FORCE)
set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -m32 -mtune=atom -mssse3 -mfpmath=sse" CACHE STRING "" FORCE)

set(CMAKE_CXX_FLAGS "${CMAKE_C_FLAGS}" CACHE STRING "" FORCE)



execute_process(COMMAND "${GCC_COMPILER}" -dumpversion OUTPUT_VARIABLE GCC_VERSION OUTPUT_STRIP_TRAILING_WHITESPACE)

set(CMAKE_EXE_LINKER_FLAGS ""            CACHE STRING "" FORCE)
set(CMAKE_EXE_LINKER_FLAGS "-Wl,--threads" CACHE STRING "" FORCE)
set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -Wl,--rpath=\"/home/nao/sysroot-${CTC_SYSROOT_VERSION}/lib\"" CACHE STRING "" FORCE)
set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -Wl,--rpath=\"/home/nao/sysroot-${CTC_SYSROOT_VERSION}/usr/lib\"" CACHE STRING "" FORCE)
set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -Wl,--dynamic-linker=\"/home/nao/sysroot-${CTC_SYSROOT_VERSION}/lib/ld-linux.so.2\"" CACHE STRING "" FORCE)

set(CMAKE_SHARED_LINKER_FLAGS "" CACHE STRING "" FORCE)
set(CMAKE_SHARED_LINKER_FLAGS "-Wl,--threads" CACHE STRING "" FORCE)
set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} -Wl,--rpath=\"/home/nao/sysroot-${CTC_SYSROOT_VERSION}/lib\"" CACHE STRING "" FORCE)
set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} -Wl,--rpath=\"/home/nao/sysroot-${CTC_SYSROOT_VERSION}/usr/lib\"" CACHE STRING "" FORCE)

set(CMAKE_MODULE_LINKER_FLAGS "" CACHE STRING "" FORCE)
set(CMAKE_MODULE_LINKER_FLAGS "-Wl,--threads" CACHE STRING "" FORCE)
set(CMAKE_MODULE_LINKER_FLAGS "${CMAKE_MODULE_LINKER_FLAGS} -Wl,--rpath=\"/home/nao/sysroot-${CTC_SYSROOT_VERSION}/lib\"" CACHE STRING "" FORCE)
set(CMAKE_MODULE_LINKER_FLAGS "${CMAKE_MODULE_LINKER_FLAGS} -Wl,--rpath=\"/home/nao/sysroot-${CTC_SYSROOT_VERSION}/usr/lib\"" CACHE STRING "" FORCE)

##
# Make sure we don't have to relink binaries when we cross-compile
set(CMAKE_BUILD_WITH_INSTALL_RPATH ON)
