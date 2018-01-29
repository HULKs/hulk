cmake_minimum_required(VERSION 3.6)
project(SimRobot)

set (CMAKE_CXX_STANDARD 14)
set (CMAKE_CXX_STANDARD_REQUIRED 14)

# Find includes in corresponding build directories
set(CMAKE_INCLUDE_CURRENT_DIR ON)
# Instruct CMake to run moc automatically when needed.
set(CMAKE_AUTOMOC ON)

#
# QT
#
find_package(Qt5 COMPONENTS Core Gui Svg Widgets)
qt5_add_resources(RESOURCES SimRobot.qrc)

#
# PThreads
#
set(CMAKE_THREAD_PREFER_PTHREAD TRUE)
set(THREADS_PREFER_PTHREAD_FLAG TRUE)
find_package(Threads REQUIRED)

set(SRC_LIST
    Main.cpp
    MainWindow.cpp
    RegisteredDockWidget.cpp
    SceneGraphDockWidget.cpp
    StatusBar.cpp
    )


add_executable(${PROJECT_NAME} ${SRC_LIST} ${RESOURCES})
target_link_libraries(${PROJECT_NAME} PRIVATE Qt5::Core Qt5::Gui Qt5::Svg Qt5::Widgets Threads::Threads)

add_dependencies(${PROJECT_NAME} SimRobotCore2 SimRobotEditor)