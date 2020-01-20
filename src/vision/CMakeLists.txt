cmake_minimum_required(VERSION 3.5)
project(Vision)

#set sources of your project here
set(HEADERS
  Modules/BallDetection/BallDetectionNeuralNet.hpp
  Modules/BallDetection/BallSeedsProvider.hpp
  Modules/BoxCandidates/BoxCandidatesProvider.hpp
  Modules/CameraCalibration/CameraCalibration.hpp
  Modules/ColorSpaceImages/ColorSpaceImagesProvider.hpp
  Modules/FieldBorderDetection/FieldBorderDetection.hpp
  Modules/FieldColorDetection/OneMeansFieldColorDetection.hpp
  Modules/FieldColorDetection/ChromaticityFieldColorDetection.hpp
  Modules/LineDetection/LineDetection.hpp
  Modules/PenaltySpotDetection/PenaltySpotDetection.hpp
  Modules/GoalDetection/GoalDetection.hpp
  Modules/ImageReceiver/FakeImageReceiver.hpp
  Modules/ImageReceiver/ImageReceiver.hpp
  Modules/ImageSegmenter/ImageSegmenter.hpp
  Modules/IntegralImageProvider/IntegralImageProvider.hpp
  Modules/LabelProvider/LabelProvider.hpp
  Modules/Projection/Projection.hpp
  Modules/Projection/ProjectionCamera.hpp
  Modules/ReplayDataProvider/ReplayDataProvider.cpp
  Modules/RobotDetection/FakeRobotDetection.hpp
  Modules/RobotDetection/RobotDetection.hpp
  Modules/RobotProjection/RobotProjectionProvider.hpp
  Modules/SaturationDetection/SaturationImageProvider.hpp
  Modules/SlidingWindows/SlidingWindowProvider.hpp
  print.hpp
)

set(SOURCES
  Modules/BallDetection/BallDetectionNeuralNet.cpp
  Modules/BallDetection/BallSeedsProvider.cpp
  Modules/BoxCandidates/BoxCandidatesProvider.cpp
  Modules/CameraCalibration/CameraCalibration.cpp
  Modules/ColorSpaceImages/ColorSpaceImagesProvider.cpp
  Modules/FieldBorderDetection/FieldBorderDetection.cpp
  Modules/FieldColorDetection/OneMeansFieldColorDetection.cpp
  Modules/FieldColorDetection/ChromaticityFieldColorDetection.cpp
  Modules/LineDetection/LineDetection.cpp
  Modules/PenaltySpotDetection/PenaltySpotDetection.cpp
  Modules/GoalDetection/GoalDetection.cpp
  Modules/ImageReceiver/FakeImageReceiver.cpp
  Modules/ImageReceiver/ImageReceiver.cpp
  Modules/ImageSegmenter/ImageSegmenter.cpp
  Modules/IntegralImageProvider/IntegralImageProvider.cpp
  Modules/LabelProvider/LabelProvider.cpp
  Modules/Projection/Projection.cpp
  Modules/Projection/ProjectionCamera.cpp
  Modules/RobotDetection/FakeRobotDetection.cpp
  Modules/ReplayDataProvider/ReplayDataProvider.hpp
  Modules/RobotDetection/RobotDetection.cpp
  Modules/RobotProjection/RobotProjectionProvider.cpp
  Modules/SaturationDetection/SaturationImageProvider.cpp
  Modules/SlidingWindows/SlidingWindowProvider.cpp
)


add_library(${PROJECT_NAME} OBJECT ${SOURCES} ${HEADERS})
target_include_directories(${PROJECT_NAME} PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

find_package(tuhhSDK REQUIRED)
target_include_directories(${PROJECT_NAME} PUBLIC ${TUHHSDK_INCLUDE_DIRS})

target_include_directories(${PROJECT_NAME} SYSTEM PUBLIC ${TUHH_DEPS_INCLUDE_DIRECTORIES})

assign_source_group(${SOURCES} ${HEADERS})
