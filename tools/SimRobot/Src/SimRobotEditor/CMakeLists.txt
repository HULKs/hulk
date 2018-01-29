cmake_minimum_required(VERSION 3.6)
project(SimRobotEditor)

set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED 14)

# Find includes in corresponding build directories
set(CMAKE_INCLUDE_CURRENT_DIR ON)
# Instruct CMake to run moc automatically when needed.
set(CMAKE_AUTOMOC ON)

#
# QT
#
find_package(Qt5 COMPONENTS Core Gui Widgets)
qt5_add_resources(RESOURCES SimRobotEditor.qrc)

set(SRC_LIST
        EditorModule.cpp
        EditorWidget.cpp
        SyntaxHighlighter.cpp
        )

add_library(${PROJECT_NAME} SHARED ${SRC_LIST} ${RESOURCES})
target_link_libraries(${PROJECT_NAME} PRIVATE Qt5::Core Qt5::Gui Qt5::Widgets)
