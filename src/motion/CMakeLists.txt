cmake_minimum_required(VERSION 3.5)
project(Motion)

#set sources of your project here
set(HEADERS
  BodyDamageProvider/BodyDamageProvider.hpp
  BodyPoseEstimation/BodyPoseEstimation.hpp
  HeadMatrixBufferProvider/HeadMatrixBufferProvider.hpp
  HeadMotion/HeadMotion.hpp
  Interface/JointCommandSender.hpp
  Interface/MotionDispatcher.hpp
  JointCalibrationProvider/JointCalibrationProvider.hpp
  JointDiffProvider/JointDiffProvider.hpp
  Jump/Jump.hpp
  Kick/Kick.hpp
  KickConfigurationProvider/KickConfigurationProvider.hpp
  LEDHandler/LEDHandler.hpp
  Motion.hpp
  MotionThread.hpp
  Odometry/IMUOdometry.hpp
  Odometry/SensorFusion.hpp
  Point/Point.hpp
  Poser/Poser.hpp
  Safety/FallManager.hpp
  SensorDataProvider/SensorDataProvider.hpp
  SitDown/SitDown.hpp
  SitUp/SitUp.hpp
  Sleep/MotionSleep.hpp
  SonarFilter/SonarFilter.hpp
  StandUp/StandUp.hpp
  Utils/DynamicMovementPrimitive/DynamicMovementPrimitive.hpp
  Utils/Interpolator/Interpolator.hpp
  Utils/MotionFile/MotionFile.hpp
  Utils/MotionFile/MotionFilePlayer.hpp
  Walking/StateHandling/ActiveOption.hpp
  Walking/StateHandling/InWalkKickOption.hpp
  Walking/StateHandling/RootOption.hpp
  Walking/StateHandling/WalkingOption.hpp
  Walking/StateHandling/WalkManState.hpp
  Walking/StateHandling/WalkOptionInterface.hpp
  Walking/UNSWalking/UNSW2014Generator.hpp
  Walking/WalkManager.hpp
  print.hpp
)

set(SOURCES
  BodyDamageProvider/BodyDamageProvider.cpp
  BodyPoseEstimation/BodyPoseEstimation.cpp
  HeadMatrixBufferProvider/HeadMatrixBufferProvider.cpp
  HeadMotion/HeadMotion.cpp
  Interface/JointCommandSender.cpp
  Interface/MotionDispatcher.cpp
  JointCalibrationProvider/JointCalibrationProvider.cpp
  JointDiffProvider/JointDiffProvider.cpp
  Jump/Jump.cpp
  Kick/Kick.cpp
  KickConfigurationProvider/KickConfigurationProvider.cpp
  LEDHandler/LEDHandler.cpp
  Motion.cpp
  MotionThread.cpp
  Odometry/IMUOdometry.cpp
  Odometry/SensorFusion.cpp
  Point/Point.cpp
  Poser/Poser.cpp
  Safety/FallManager.cpp
  SensorDataProvider/SensorDataProvider.cpp
  SitDown/SitDown.cpp
  SitUp/SitUp.cpp
  SonarFilter/SonarFilter.cpp
  StandUp/StandUp.cpp
  Utils/DynamicMovementPrimitive/DynamicMovementPrimitive.cpp
  Utils/Interpolator/Interpolator.cpp
  Utils/MotionFile/MotionFile.cpp
  Utils/MotionFile/MotionFilePlayer.cpp
  Walking/UNSWalking/UNSW2014Generator.cpp
  Walking/WalkManager.cpp
  print.cpp
)


add_library(${PROJECT_NAME} OBJECT ${SOURCES} ${HEADERS})
target_include_directories(${PROJECT_NAME} PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

find_package(tuhhSDK REQUIRED)
target_include_directories(${PROJECT_NAME} PUBLIC ${TUHHSDK_INCLUDE_DIRS})

target_include_directories(${PROJECT_NAME} SYSTEM PUBLIC ${TUHH_DEPS_INCLUDE_DIRECTORIES})

assign_source_group(${SOURCES} ${HEADERS})
