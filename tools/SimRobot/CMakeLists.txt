cmake_minimum_required(VERSION 3.1)
project(simrobot)

# Set CMAKE_MODULE_PATH to assist cmake in finding our own package definitions
set(CMAKE_MODULE_PATH ${CMAKE_MODULE_PATH} "${CMAKE_SOURCE_DIR}/CMake")

# Define some macros so the code has the possibility to find out what it is
# being compiled for (even though it should not be necessary most times).

# SimRobot controllers are shared libraries. The object libraries need to
# know that.
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

set (CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR})
set (CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR})

find_package(OpenMP)
if (OPENMP_FOUND)
    set (CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${OpenMP_C_FLAGS} -D_GLIBCXX_PARALLEL")
    set (CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${OpenMP_CXX_FLAGS} -D_GLIBCXX_PARALLEL")
endif()

if (UNIX)
    add_definitions("-D __extern_always_inline=inline -D LINUX -D dDOUBLE -D dIDEDOUBLE")
    set(EXTRA_FLAGS "-march=native -fPIC -pipe -Wall -Wsign-compare")
    set (CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${EXTRA_FLAGS}")
    set (CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${EXTRA_FLAGS}")
    set (CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -D _DEBUG")
    set (CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} -fomit-frame-pointer -DNDEBUG")


elseif (WIN32)
    add_definitions("-D NOMINMAX -D WINDOWS -D _CRT_SECURE_NO_DEPRECATE -D D_SCL_SECURE_NO_WARNINGS")
    set(EXTRA_FLAGS "/W3 /GF /Gm- /MP /Zc:wchar_t-")
    set (CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${EXTRA_FLAGS}")
    set (CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${EXTRA_FLAGS}")
    set (CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} /Od /ZI -D _DEBUG")
    set (CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} /Ox /Ob2 /Oi /Ot /Oy /GT -D NDEBUG")
endif()


add_subdirectory(Src/SimRobot)
add_subdirectory(Src/SimRobotCore2)
add_subdirectory(Src/SimRobotEditor)

