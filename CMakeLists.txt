cmake_minimum_required(VERSION 3.16)
project(
  hulk
  LANGUAGES CXX
)

# Make project modules available
list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_LIST_DIR}/cmake")

# Register build types Develop and DevWithDebInfo
if(CMAKE_CONFIGURATION_TYPES)
  if(NOT "Develop" IN_LIST CMAKE_CONFIGURATION_TYPES)
    list(APPEND CMAKE_CONFIGURATION_TYPES Develop)
  endif()
  if(NOT "DevWithDebInfo" IN_LIST CMAKE_CONFIGURATION_TYPES)
    list(APPEND CMAKE_CONFIGURATION_TYPES DevWithDebInfo)
  endif()
else()
  set(allowableBuildTypes Debug Develop DevWithDebInfo Release RelWithDebInfo)
  set_property(CACHE CMAKE_BUILD_TYPE PROPERTY STRINGS "${allowableBuildTypes}")
  if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Develop CACHE STRING "" FORCE)
  elseif(NOT CMAKE_BUILD_TYPE IN_LIST allowableBuildTypes)
    message(FATAL_ERROR "Invalid build type: ${CMAKE_BUILD_TYPE}")
  endif()
endif()

# Target for which the build system is generated
set(HULK_TARGET "" CACHE STRING "Either NAO, Replay, SimRobot, or Webots")
if(NOT ("${HULK_TARGET}" MATCHES "^(NAO|Replay|SimRobot|Webots)$"))
  message(FATAL_ERROR "TARGET must be either NAO, Replay, SimRobot, or Webots")
endif()

# Options for features of the code
if(("${HULK_TARGET}" STREQUAL "SimRobot") OR ("${HULK_TARGET}" STREQUAL "Webots"))
  option(HULK_UNIX_SOCKET "Enable the implementation of unix sockets for debug communication" ON)
else()
  option(HULK_UNIX_SOCKET "Enable the implementation of unix sockets for debug communication" OFF)
endif()

# Generate compile_commands.json
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Find packages that are used in all targets
find_package(Boost REQUIRED)
find_package(CompiledNN REQUIRED)
find_package(Eigen3 3.3 REQUIRED) # 3.3 because earlier normalize(d) would be NaN when the vector to be normalized is 0
find_package(FFTW3 REQUIRED)
find_package(JPEG REQUIRED)
find_package(opusfile REQUIRED)
find_package(PNG REQUIRED)
find_package(Threads REQUIRED)
find_package(ZLIB REQUIRED)

# Find packages for specific targets
if("${HULK_TARGET}" STREQUAL "NAO")
  find_package(ITTNotify)
  find_package(ALSA REQUIRED)
elseif("${HULK_TARGET}" STREQUAL "SimRobot")
  find_package(portaudio REQUIRED)
  find_package(Qt5 COMPONENTS Core Widgets REQUIRED)
elseif("${HULK_TARGET}" STREQUAL "Webots")
  find_package(Webots REQUIRED)
endif()

# Register executable/library targets (will be populated via target_sources() in subdirectories)
if("${HULK_TARGET}" STREQUAL "NAO")
  add_executable(hulk)
elseif("${HULK_TARGET}" STREQUAL "Replay")
  add_executable(hulk)
  set_target_properties(hulk PROPERTIES OUTPUT_NAME "replay")
elseif("${HULK_TARGET}" STREQUAL "SimRobot")
  set(CMAKE_AUTOMOC ON)
  add_library(hulk SHARED)
elseif("${HULK_TARGET}" STREQUAL "Webots")
  add_executable(hulk)
endif()

# Enable CCache
find_program(CCACHE_PROGRAM ccache)
if(CCACHE_PROGRAM)
  set_property(TARGET hulk PROPERTY RULE_LAUNCH_COMPILE "${CCACHE_PROGRAM}")
endif()

# Compilation flags
target_compile_options(hulk PRIVATE
  "$<$<CXX_COMPILER_ID:GNU,Clang>:-Wall>"
  "$<$<CXX_COMPILER_ID:GNU,Clang>:-Wextra>"
  "$<$<CXX_COMPILER_ID:GNU,Clang>:-pedantic>"
  "$<$<CXX_COMPILER_ID:GNU,Clang>:-pipe>"
  "$<$<CXX_COMPILER_ID:GNU,Clang>:-mssse3>"
  "$<$<AND:$<CXX_COMPILER_ID:GNU,Clang>,$<OR:$<CONFIG:Debug>,$<CONFIG:DevWithDebInfo>>>:-g>"
  "$<$<AND:$<CXX_COMPILER_ID:GNU,Clang>,$<OR:$<CONFIG:Debug>,$<CONFIG:Develop>,$<CONFIG:DevWithDebInfo>>>:-pedantic-errors>"
  "$<$<AND:$<CXX_COMPILER_ID:GNU,Clang>,$<OR:$<CONFIG:Release>,$<CONFIG:RelWithDebInfo>>>:-fomit-frame-pointer>"
  "$<$<AND:$<CXX_COMPILER_ID:GNU,Clang>,$<OR:$<CONFIG:RelWithDebInfo>>>:-ggdb>"
  "$<$<AND:$<CXX_COMPILER_ID:GNU,Clang>,$<OR:$<CONFIG:Develop>,$<CONFIG:Release>,$<CONFIG:RelWithDebInfo>,$<CONFIG:DevWithDebInfo>>>:-O3>"
  "$<$<AND:$<CXX_COMPILER_ID:GNU,Clang>,$<OR:$<CONFIG:Debug>,$<CONFIG:Develop>,$<CONFIG:DevWithDebInfo>>>:-fno-omit-frame-pointer>"
)
target_compile_definitions(hulk PRIVATE
  "HULK_TARGET_${HULK_TARGET}"
  "$<$<AND:$<CXX_COMPILER_ID:GNU,Clang>,$<OR:$<CONFIG:Release>,$<CONFIG:RelWithDebInfo>>>:NDEBUG>"
)
if("${HULK_TARGET}" STREQUAL "NAO")
  target_compile_definitions(hulk PRIVATE
    "EIGEN_DONT_VECTORIZE"
    "EIGEN_DISABLE_UNALIGNED_ARRAY_ASSERT"
  )
endif()

# Use the llvm gold linker
target_link_options(hulk PRIVATE "-fuse-ld=gold")

# Set C++ standard and disable C++ extensions
set_property(TARGET hulk PROPERTY CXX_STANDARD 17)
set_property(TARGET hulk PROPERTY CXX_STANDARD_REQUIRED ON)
target_compile_features(hulk PRIVATE cxx_std_17)
set_property(TARGET hulk PROPERTY CXX_EXTENSIONS OFF)

# Compile with unity builds
set_property(TARGET hulk PROPERTY UNITY_BUILD ON)

# Add SimRobot include directory and enable PIC for shared library
if("${HULK_TARGET}" STREQUAL "SimRobot")
  target_include_directories(hulk PRIVATE
    "${CMAKE_SOURCE_DIR}/tools/SimRobot/Src/SimRobotCore2"
  )
  set_property(TARGET hulk PROPERTY CMAKE_POSITION_INDEPENDENT_CODE ON)
endif()

# Make options available as preprocessor defines
if (HULK_UNIX_SOCKET)
  target_compile_definitions(hulk PRIVATE
    "HULK_UNIX_SOCKET"
  )
endif()

# Link libraries that are used by all targets
target_link_libraries(hulk PRIVATE
  ${CMAKE_DL_LIBS}
  Boost::headers
  CompiledNN::CompiledNN
  Eigen3::Eigen
  FFTW3::fftw3
  JPEG::JPEG
  opusfile::opusfile
  PNG::PNG
  Threads::Threads
  ZLIB::ZLIB
)

# Link libraries that are only used by specific targets
if("${HULK_TARGET}" STREQUAL "NAO")
  target_link_libraries(hulk PRIVATE
    ALSA::ALSA
    rt
  )
  if(${ITTNOTIFY_FOUND})
    target_compile_definitions(hulk PRIVATE
      "ITTNOTIFY_FOUND"
    )
    target_link_libraries(hulk PRIVATE
      "${ITTNOTIFY_LIBRARIES}"
    )
    target_include_directories(hulk PRIVATE
      "${ITTNOTIFY_INCLUDE_DIRS}"
    )
  endif()
elseif("${HULK_TARGET}" STREQUAL "SimRobot")
  target_link_libraries(hulk PRIVATE
    portaudio::portaudio
    Qt5::Core
    Qt5::Widgets
  )
elseif("${HULK_TARGET}" STREQUAL "Webots")
  target_link_libraries(hulk PRIVATE
    Webots::Webots
  )
  get_target_property(LIBS Webots::Webots INTERFACE_LINK_LIBRARIES)
endif()

# Setup LOCAL_FILE_ROOT
if(DEFINED ENV{LOCAL_FILE_ROOT})
  target_compile_definitions(hulk PRIVATE
    LOCAL_FILE_ROOT="$ENV{LOCAL_FILE_ROOT}"
  )
else()
  if(("${HULK_TARGET}" STREQUAL "Replay") OR ("${HULK_TARGET}" STREQUAL "SimRobot") OR ("${HULK_TARGET}" STREQUAL "Webots"))
    target_compile_definitions(hulk PRIVATE
      LOCAL_FILE_ROOT="${CMAKE_SOURCE_DIR}/webots/"
    )
  endif()
endif()

# Add src/ directory with all of the code
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/src")
