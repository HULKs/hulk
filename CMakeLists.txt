cmake_minimum_required(VERSION 3.5)
if(NAO)
  if(NOT TOOLCHAIN_DIR)
    set(TOOLCHAIN_DIR "${CMAKE_SOURCE_DIR}/toolchain")
  endif(NOT TOOLCHAIN_DIR)
  set(CMAKE_TOOLCHAIN_FILE "${TOOLCHAIN_DIR}/hulks/toolchain.cmake")
  set(CMAKE_PREFIX_PATH "${TOOLCHAIN_DIR}/hulks/root")
elseif(SIMROBOT)
  if(NOT SIMROBOT_INCLUDE_DIRS)
    message(FATAL_ERROR "The simrobot target needs SIMROBOT_INCLUDE_DIRS defined in CMake.")
  endif(NOT SIMROBOT_INCLUDE_DIRS)
endif(NAO)
project(tuhhnao)

# Set CMAKE_MODULE_PATH to assist cmake in finding our own package definitions
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${CMAKE_SOURCE_DIR}/CMake")
set(CMAKE_EXPORT_COMPILE_COMMANDS 1)

include(CMake/settings.cmake)
include(CMake/dependencies.cmake)

# Define some macros so the code has the possibility to find out what it is
# being compiled for (even though it should not be necessary most times).
if(NAO)
  add_definitions(-DNAO)
elseif(REPLAY)
  add_definitions(-DREPLAY)
elseif(SIMROBOT)
  add_definitions(-DSIMROBOT)
  # SimRobot controllers are shared libraries. The object libraries need to
  # know that.
  set(CMAKE_POSITION_INDEPENDENT_CODE ON)
elseif(QT_WEBSOCKET)
  add_definitions(-DQT_WEBSOCKET)
endif(NAO)

add_subdirectory(src/tuhhsdk)
add_subdirectory(src/brain)
add_subdirectory(src/vision)
add_subdirectory(src/motion)

if(IDE)
  add_subdirectory(../tools "${CMAKE_CURRENT_BINARY_DIR}/tools")
endif(IDE)
