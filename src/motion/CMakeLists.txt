cmake_minimum_required(VERSION 3.5)
project(Motion)

#set sources of your project here
set(HEADERS
  print.hpp
  Motion.hpp
  MotionThread.hpp
  StandUp/StandUp.hpp
  Walking/WalkingEngine.hpp
  Walking/Pendulum.hpp
  Walking/FootController.hpp
  Walking/StepPlanner.hpp
  Safety/FallManager.hpp
  SensorDataProvider/SensorDataProvider.hpp
  Utils/DynamicMovementPrimitive/DynamicMovementPrimitive.hpp
  Utils/MotionFile/MotionFile.hpp
  Utils/MotionFile/MotionFilePlayer.hpp
  Utils/Interpolator/Interpolator.hpp
  Poser/Poser.hpp
  Kick/Kick.hpp
  Kick/KickPhase.hpp
  LEDHandler/LEDHandler.hpp
  Point/Point.hpp
  Keeper/Keeper.hpp
  HeadMatrixBufferProvider/HeadMatrixBufferProvider.hpp
  HeadMotion/HeadMotion.hpp
  BodyPoseEstimation/BodyPoseEstimation.hpp
  Odometry/IMUOdometry.hpp
  Odometry/SensorFusion.hpp
  SonarFilter/SonarFilter.hpp
  Interface/MotionDispatcher.hpp
  Interface/JointCommandSender.hpp
)

set(SOURCES
  Motion.cpp
  MotionThread.cpp
  print.cpp
  StandUp/StandUp.cpp
  Safety/FallManager.cpp
  Walking/WalkingEngine.cpp
  Walking/Pendulum.cpp
  Walking/FootController.cpp
  Walking/StepPlanner.cpp
  SensorDataProvider/SensorDataProvider.cpp
  Kick/Kick.cpp
  Kick/KickPhase.cpp
  LEDHandler/LEDHandler.cpp
  Point/Point.cpp
  Utils/DynamicMovementPrimitive/DynamicMovementPrimitive.cpp
  Utils/MotionFile/MotionFile.cpp
  Utils/MotionFile/MotionFilePlayer.cpp
  Utils/Interpolator/Interpolator.cpp
  Poser/Poser.cpp
  Keeper/Keeper.cpp
  HeadMatrixBufferProvider/HeadMatrixBufferProvider.cpp
  HeadMotion/HeadMotion.cpp
  BodyPoseEstimation/BodyPoseEstimation.cpp
  Odometry/IMUOdometry.cpp
  Odometry/SensorFusion.cpp
  SonarFilter/SonarFilter.cpp
  Interface/MotionDispatcher.cpp
  Interface/JointCommandSender.cpp
)


add_library(${PROJECT_NAME} OBJECT ${SOURCES} ${HEADERS})
target_include_directories(${PROJECT_NAME} PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

find_package(tuhhSDK REQUIRED)
target_include_directories(${PROJECT_NAME} PUBLIC ${TUHHSDK_INCLUDE_DIRS})

target_include_directories(${PROJECT_NAME} SYSTEM PUBLIC ${TUHH_DEPS_INCLUDE_DIRECTORIES})

assign_source_group(${SOURCES} ${HEADERS})
