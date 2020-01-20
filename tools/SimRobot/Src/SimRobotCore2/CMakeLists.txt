cmake_minimum_required(VERSION 3.6)
project(SimRobotCore2)

set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED 14)

# Find includes in corresponding build directories
set(CMAKE_INCLUDE_CURRENT_DIR ON)
# Instruct CMake to run moc automatically when needed.
set(CMAKE_AUTOMOC ON)

find_package(Qt5 COMPONENTS Core Gui OpenGL Widgets)
qt5_add_resources(RESOURCES SimRobotCore2.qrc)
find_package(LibXml2 REQUIRED)
find_package(ODE REQUIRED)
find_package(GLEW REQUIRED)
find_package(OpenGL REQUIRED)
set(CMAKE_THREAD_PREFER_PTHREAD TRUE)
set(THREADS_PREFER_PTHREAD_FLAG TRUE)
find_package(Threads REQUIRED)

find_package(Eigen3 3.3 REQUIRED NO_MODULE)

set(DEPS_INCLUDE_DIRECTORIES
        ${LIBXML2_INCLUDE_DIR}
        ${ODE_INCLUDE_DIRS}
        ${OPENGL_INCLUDE_DIR}
        )

set(LIBS
        ${LIBXML2_LIBRARIES}
        ${ODE_LIBRARIES}
        Qt5::Core
        Qt5::Gui
        Qt5::OpenGL
        Qt5::Widgets
        GLEW::GLEW
        ${OPENGL_LIBRARIES}
        Threads::Threads
        Eigen3::Eigen
        )

file (GLOB_RECURSE SOURCES ${PROJECT_SOURCE_DIR}/*.cpp)
file (GLOB_RECURSE HEADERS ${PROJECT_SOURCE_DIR}/*.h)

add_library(${PROJECT_NAME} SHARED ${SOURCES} ${HEADERS} ${RESOURCES})
target_link_libraries(${PROJECT_NAME} ${LIBS})
target_include_directories(${PROJECT_NAME} PRIVATE ${DEPS_INCLUDE_DIRECTORIES})
