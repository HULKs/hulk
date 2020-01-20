cmake_minimum_required(VERSION 3.5)

option(NAO "Compile the code for the nao" OFF)
option(NAO_V5 "Compile the code for the naov5 and lower" OFF)
option(NAO_V6 "Compile the code for the naov6" OFF)
option(NAOLIB "If compiling for nao, should we build tuhhALModule" ON)
option(NAONATIVE "If compiling for nao, shall the crosstoolchain be used" ON)
option(REPLAY "Build for replay" OFF)
option(SIMROBOT "Build for simrobot" OFF)
option(QT_WEBSOCKET "Build for qtwebsockets" OFF)
option(IDE "Include the tools repo into the list of files" OFF)

if(NAO_V5 OR NAO_V6)
  set(NAO ON)
endif(NAO_V5 OR NAO_V6)

if(NAO_V6)
  set(NAOLIB OFF)
endif(NAO_V6)

if(NAO AND NAONATIVE)
  if(NOT TOOLCHAIN_DIR)
    set(TOOLCHAIN_DIR "${CMAKE_SOURCE_DIR}/toolchain")
  endif(NOT TOOLCHAIN_DIR)
  if(NAO_V5)
    set(CMAKE_TOOLCHAIN_FILE "${TOOLCHAIN_DIR}/hulks-v5/toolchain.cmake")
    set(CMAKE_PREFIX_PATH "${TOOLCHAIN_DIR}/hulks-v5/root")
  elseif(NAO_V6)
    set(CMAKE_TOOLCHAIN_FILE "${TOOLCHAIN_DIR}/hulks-v6/toolchain.cmake")
    set(CMAKE_PREFIX_PATH "${TOOLCHAIN_DIR}/hulks-v6/root")
  endif(NAO_V5)
elseif(SIMROBOT)
  if(NOT SIMROBOT_INCLUDE_DIRS)
    set(SIMROBOT_INCLUDE_DIRS "${CMAKE_SOURCE_DIR}/tools/SimRobot/Src/SimRobotCore2")
  endif(NOT SIMROBOT_INCLUDE_DIRS)
endif(NAO AND NAONATIVE)

if(NOT NAO)
  find_program(CCACHE_FOUND ccache)
  if(CCACHE_FOUND)
      set_property(GLOBAL PROPERTY RULE_LAUNCH_COMPILE ccache)
      set_property(GLOBAL PROPERTY RULE_LAUNCH_LINK ccache)
  endif(CCACHE_FOUND)
endif(NOT NAO)

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
  if(NAO_V6)
    add_definitions(-DNAOV6)
  elseif(NAO_V5)
    add_definitions(-DNAOV5)
  endif(NAO_V6)
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
