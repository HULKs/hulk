cmake_minimum_required(VERSION 3.5)
project(Vision)

#set sources of your project here
set(HEADERS
  print.hpp
  Modules/FieldColorDetection/OneMeansFieldColorDetection.hpp
  Modules/FieldBorderDetection/FieldBorderDetection.hpp
  Modules/BallDetection/BallDetectionNeuralNet.hpp
  Modules/GoalDetection/GoalDetection.hpp
  Modules/ImageSegmenter/ImageSegmenter.hpp
  Modules/CameraCalibration/CameraCalibration.hpp
  Modules/FieldMarksDetection/FieldMarksDetection.hpp
  Modules/ImageReceiver/ImageReceiver.hpp
  Modules/Projection/Projection.hpp
  Modules/Projection/ProjectionCamera.hpp
  Modules/RobotProjection/RobotProjectionProvider.hpp
  Utils/Algorithms.hpp
)

set(SOURCES
  Modules/FieldColorDetection/OneMeansFieldColorDetection.cpp
  Modules/FieldBorderDetection/FieldBorderDetection.cpp
  Modules/BallDetection/BallDetectionNeuralNet.cpp
  Modules/GoalDetection/GoalDetection.cpp
  Modules/ImageSegmenter/ImageSegmenter.cpp
  Modules/CameraCalibration/CameraCalibration.cpp
  Modules/FieldMarksDetection/FieldMarksDetection.cpp
  Modules/ImageReceiver/ImageReceiver.cpp
  Modules/Projection/Projection.cpp
  Modules/Projection/ProjectionCamera.cpp
  Modules/RobotProjection/RobotProjectionProvider.cpp
)


add_library(${PROJECT_NAME} OBJECT ${SOURCES} ${HEADERS})
target_include_directories(${PROJECT_NAME} PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

find_package(tuhhSDK REQUIRED)
target_include_directories(${PROJECT_NAME} PUBLIC ${TUHHSDK_INCLUDE_DIRS})

target_include_directories(${PROJECT_NAME} SYSTEM PUBLIC ${TUHH_DEPS_INCLUDE_DIRECTORIES})

assign_source_group(${SOURCES} ${HEADERS})
