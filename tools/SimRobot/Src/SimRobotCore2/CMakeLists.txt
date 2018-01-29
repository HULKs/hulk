cmake_minimum_required(VERSION 3.6)
project(SimRobotCore2)

set(CMAKE_CXX_STANDARD 14)
set(CMAKE_CXX_STANDARD_REQUIRED 14)

# Find includes in corresponding build directories
set(CMAKE_INCLUDE_CURRENT_DIR ON)
# Instruct CMake to run moc automatically when needed.
set(CMAKE_AUTOMOC ON)

#
# QT
#
find_package(Qt5 COMPONENTS Core Gui OpenGL Widgets)
qt5_add_resources(RESOURCES SimRobotCore2.qrc)

#
# Libxml
#
find_package(LibXml2 REQUIRED)

#
# Ode
#
find_package(ODE REQUIRED)

#
# OpenGl
#
find_package(GLEW REQUIRED)
find_package(OpenGL REQUIRED)

#
# PThreads
#
set(CMAKE_THREAD_PREFER_PTHREAD TRUE)
set(THREADS_PREFER_PTHREAD_FLAG TRUE)
find_package(Threads REQUIRED)


###############################################################################
# Include directories
###############################################################################
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
        )

set(SRC_LIST
        Parser/Element.cpp
        Parser/Parser.cpp
        Parser/Reader.cpp

        Platform/Assert.cpp
        Platform/OffscreenRenderer.cpp
        Platform/System.cpp

        Simulation/Actuators/Actuator.cpp
        Simulation/Actuators/Hinge.cpp
        Simulation/Actuators/Joint.cpp
        Simulation/Actuators/Slider.cpp

        Simulation/Appearances/Appearance.cpp
        Simulation/Appearances/BoxAppearance.cpp
        Simulation/Appearances/CapsuleAppearance.cpp
        Simulation/Appearances/ComplexAppearance.cpp
        Simulation/Appearances/CylinderAppearance.cpp
        Simulation/Appearances/SphereAppearance.cpp

        Simulation/Geometries/BoxGeometry.cpp
        Simulation/Geometries/CapsuleGeometry.cpp
        Simulation/Geometries/CylinderGeometry.cpp
        Simulation/Geometries/Geometry.cpp
        Simulation/Geometries/SphereGeometry.cpp

        Simulation/Masses/BoxMass.cpp
        Simulation/Masses/InertiaMatrixMass.cpp
        Simulation/Masses/Mass.cpp
        Simulation/Masses/SphereMass.cpp

        Simulation/Motors/ServoMotor.cpp
        Simulation/Motors/VelocityMotor.cpp

        Simulation/Sensors/Accelerometer.cpp
        Simulation/Sensors/ApproxDistanceSensor.cpp
        Simulation/Sensors/Camera.cpp
        Simulation/Sensors/CollisionSensor.cpp
        Simulation/Sensors/DepthImageSensor.cpp
        Simulation/Sensors/Gyroscope.cpp
        Simulation/Sensors/Sensor.cpp
        Simulation/Sensors/SingleDistanceSensor.cpp

        Simulation/Axis.cpp
        Simulation/Body.cpp
        Simulation/Compound.cpp
        Simulation/GraphicalObject.cpp
        Simulation/PhysicalObject.cpp
        Simulation/Scene.cpp
        Simulation/SimObject.cpp
        Simulation/Simulation.cpp
        Simulation/UserInput.cpp

        Tools/Matrix3x3.cpp
        Tools/ODETools.h
        Tools/OpenGLTools.cpp
        Tools/Texture.cpp
        Tools/Vector3.cpp

        ActuatorsWidget.cpp
        CoreModule.cpp
        SensorWidget.cpp
        SimObjectRenderer.cpp
        SimObjectWidget.cpp
        )

add_library(${PROJECT_NAME} SHARED ${SRC_LIST} ${RESOURCES})
target_link_libraries(${PROJECT_NAME} ${LIBS})
target_include_directories(${PROJECT_NAME} PRIVATE ${DEPS_INCLUDE_DIRECTORIES})
