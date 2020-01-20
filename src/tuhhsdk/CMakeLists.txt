cmake_minimum_required(VERSION 3.5)
project(tuhh)

#set sources of your project here
set(TUHHSDK_SOURCES
  Data/MirrorableMotionOutput.cpp
  Definitions/DevilSmashStandardMessage.cpp
  Definitions/DevilSmashStandardMessageTest.cpp
  Definitions/keys.cpp
  Framework/Database.cpp
  Framework/DebugDatabase.cpp
  Framework/Messaging.cpp
  Framework/Module.cpp
  Framework/ModuleManagerInterface.cpp
  Framework/Thread.cpp
  Libs/jsoncpp.cpp
  Modules/Configuration/Configuration.cpp
  Modules/Configuration/NetworkConfig.cpp
  Modules/Configuration/UnixSocketConfig.cpp
  Modules/Debug/Debug.cpp
  Modules/Debug/FileTransport.cpp
  Modules/Debug/JpegConverter.cpp
  Modules/Debug/PngConverter.cpp
  Modules/Debug/StreamCompression.cpp
  Modules/Debug/TCPTransport.cpp
  Modules/Debug/UnixSocketTransport.cpp
  Modules/MachineLearning/NeuralNetwork/NeuralNetwork.cpp
  Modules/NaoProvider.cpp
  Modules/Network/AlivenessTransmitter.cpp
  Modules/Poses.cpp
  print.cpp
  SharedObject.cpp
  SharedObjectManager.cpp
  Tools/Chronometer.cpp
  Tools/Kinematics/Com.cpp
  Tools/Kinematics/ForwardKinematics.cpp
  Tools/Kinematics/InverseKinematics.cpp
  Tools/Math/ColorConverter.cpp
  Tools/Math/Correlation.cpp
  Tools/Math/FFT.cpp
  Tools/Math/HungarianMethod.cpp
  Tools/Math/Random.cpp
  Tools/Storage/Image.cpp
  Tools/Storage/Image422.cpp
  Tools/Storage/UniValue/UniValue.cpp
  Tools/Storage/UniValue/UniValue2JsonString.cpp
  Tools/Storage/XPMImage.cpp
  Tools/Time.cpp
  tuhh.cpp
)

set(TUHHSDK_HEADERS
  Data/AudioData.hpp
  Data/BallData.hpp
  Data/BallSearchMap.hpp
  Data/BallSearchPosition.hpp
  Data/BallState.hpp
  Data/BallSeeds.hpp
  Data/BatteryData.hpp
  Data/BoxCandidates.hpp
  Data/BishopPosition.hpp
  Data/BodyPose.hpp
  Data/BodyRotationData.hpp
  Data/ButtonData.hpp
  Data/CameraMatrix.hpp
  Data/CircleData.hpp
  Data/CycleInfo.hpp
  Data/DefenderAction.hpp
  Data/DefendingPosition.hpp
  Data/EyeLEDRequest.hpp
  Data/FakeImageData.hpp
  Data/FallManagerOutput.hpp
  Data/FieldBorder.hpp
  Data/FieldColor.hpp
  Data/FieldDimensions.hpp
  Data/FilteredRobots.hpp
  Data/FilteredSegments.hpp
  Data/FreeKickPose.hpp
  Data/FSRSensorData.hpp
  Data/GameControllerState.hpp
  Data/GoalData.hpp
  Data/HeadPositionData.hpp
  Data/HeadMatrixBuffer.hpp
  Data/HeadMotionOutput.hpp
  Data/IMUSensorData.hpp
  Data/ImageData.hpp
  Data/ImageSegments.hpp
  Data/IntegralImageData.hpp
  Data/JointCalibrationData.hpp
  Data/JointSensorData.hpp
  Data/KeeperAction.hpp
  Data/JumpAction.hpp
  Data/JumpOutput.hpp
  Data/KickConfigurationData.hpp
  Data/KickOutput.hpp
  Data/LandmarkModel.hpp
  Data/LineData.hpp
  Data/MirrorableMotionOutput.hpp
  Data/MotionActivation.hpp
  Data/MotionOutput.hpp
  Data/MotionPlannerOutput.hpp
  Data/MotionRequest.hpp
  Data/MotionState.hpp
  Data/NetworkServiceData.hpp
  Data/NTPData.hpp
  Data/ObstacleData.hpp
  Data/OdometryData.hpp
  Data/OdometryOffset.hpp
  Data/Path.hpp
  Data/PenaltySpotData.hpp
  Data/PenaltyKeeperAction.hpp
  Data/PlayerConfiguration.hpp
  Data/PlayingRoles.hpp
  Data/PointOfInterests.hpp
  Data/PointOutput.hpp
  Data/PoserOutput.hpp
  Data/RawTeamPlayers.hpp
  Data/ReplayData.hpp
  Data/RobotData.hpp
  Data/RobotKinematics.hpp
  Data/RobotPosition.hpp
  Data/RobotProjection.hpp
  Data/SPLNetworkData.hpp
  Data/SetPosition.hpp
  Data/SlidingWindows.hpp
  Data/SonarData.hpp
  Data/StandUpOutput.hpp
  Data/StandUpResult.hpp
  Data/StrikerAction.hpp
  Data/SupportingPosition.hpp
  Data/TeamBallModel.hpp
  Data/TeamObstacleData.hpp
  Data/TeamPlayers.hpp
  Data/TimeToReachBall.hpp
  Data/WalkGenerator.hpp
  Data/WalkingEngineStandOutput.hpp
  Data/WalkingEngineWalkOutput.hpp
  Data/WhistleData.hpp
  Data/WorldState.hpp
  Definitions/DevilSmashStandardMessage.hpp
  Definitions/DevilSmashStandardMessageTest.hpp
  Definitions/RoboCupGameControlData.h
  Definitions/SPLCoachMessage.h
  Definitions/SPLStandardMessage.h
  Definitions/XPMImages.h
  Definitions/keys.h
  Definitions/windows_definition_fix.hpp
  Framework/DataType.hpp
  Framework/DebugDatabase.hpp
  Framework/Database.hpp
  Framework/Messaging.hpp
  Framework/Module.hpp
  Framework/ModuleManagerInterface.hpp
  Framework/Thread.hpp
  Hardware/AudioInterface.hpp
  Hardware/CameraInterface.hpp
  Hardware/FakeDataInterface.hpp
  Hardware/RobotInterface.hpp
  Libs/json/json-forwards.h
  Libs/json/json.h
  Modules/Configuration/ConfigMessageHeader.h
  Modules/Configuration/Configuration.h
  Modules/Configuration/NetworkConfig.hpp
  Modules/Configuration/UnixSocketConfig.hpp
  Modules/Debug/Debug.h
  Modules/Debug/DebugData.h
  Modules/Debug/DebugMessageFormat.h
  Modules/Debug/DebugTransportInterface.h
  Modules/Debug/FileTransport.h
  Modules/Debug/ImageConverterInterface.h
  Modules/Debug/JpegConverter.h
  Modules/Debug/PngConverter.h
  Modules/Debug/StreamCompression.h
  Modules/Debug/Subscription.h
  Modules/Debug/TCPTransport.h
  Modules/Debug/UnixSocketTransport.hpp
  Modules/Log/Log.h
  Modules/MachineLearning/NeuralNetwork/NeuralNetwork.hpp
  Modules/NaoProvider.h
  Modules/Network/AlivenessMessage.h
  Modules/Network/AlivenessTransmitter.h
  Modules/Poses.h
  SharedObject.hpp
  SharedObjectManager.hpp
  Tools/Backtrace/Backtrace.hpp
  Tools/BallUtils.hpp
  Tools/Chronometer.hpp
  Tools/Kinematics/Com.h
  Tools/Kinematics/ForwardKinematics.h
  Tools/Kinematics/InverseKinematics.h
  Tools/Kinematics/KinematicMatrix.h
  Tools/Math/Angle.hpp
  Tools/Math/Arc.hpp
  Tools/Math/Circle.hpp
  Tools/Math/ColorConverter.hpp
  Tools/Math/Correlation.hpp
  Tools/Math/Eigen.hpp
  Tools/Math/FFT.hpp
  Tools/Math/Geometry.hpp
  Tools/Math/HungarianMethod.hpp
  Tools/Math/Hysteresis.hpp
  Tools/Math/Line.hpp
  Tools/Math/MovingAverage.hpp
  Tools/Math/Plane.hpp
  Tools/Math/Polygon.hpp
  Tools/Math/ConvexPolygon.hpp
  Tools/Math/Pose.hpp
  Tools/Math/Random.hpp
  Tools/Math/Range.hpp
  Tools/Math/Statistics.hpp
  Tools/Math/Velocity.hpp
  Tools/StateEstimation/Covariance.hpp
  Tools/StateEstimation/ProjectionMeasurementModel.hpp
  Tools/StateEstimation/KalmanFilter.hpp
  Tools/StateEstimation/KalmanFilter_impl.hpp
  Tools/StateEstimation/UKF.hpp
  Tools/StateEstimation/UKF_Impl.hpp
  Tools/StateMachine/Option.hpp
  Tools/Storage/Image.hpp
  Tools/Storage/Image422.hpp
  Tools/Storage/ObjectCandidate.hpp
  Tools/Storage/UniValue/EigenStreaming.hpp
  Tools/Storage/UniValue/UniValue.h
  Tools/Storage/UniValue/UniValue2Json.hpp
  Tools/Storage/UniValue/UniValue2JsonString.h
  Tools/Storage/XPMImage.hpp
  Tools/Time.hpp
  Tools/Var/SpscQueue.hpp
  print.h
  tuhh.hpp
)

set(NAO_SOURCES
  Hardware/Nao/common/main.cpp
  Hardware/Nao/common/NaoAudio.cpp
  Hardware/Nao/common/NaoCamera.cpp
  Hardware/Nao/common/NaoFakeData.cpp
  Hardware/Nao/common/BatteryDisplay.cpp
  Hardware/Nao/common/SMO.cpp
  Hardware/Nao/common/NaoCamera.cpp
  Hardware/Nao/common/NaoAudio.cpp
  ${TUHHSDK_SOURCES}
)

set(NAO_HEADERS
  Hardware/Nao/common/NaoAudio.hpp
  Hardware/Nao/common/NaoCamera.hpp
  Hardware/Nao/common/NaoCameraCommon.hpp
  Hardware/Nao/common/NaoFakeData.hpp
  Hardware/Nao/v6/Nao6Interface.hpp
  Hardware/Nao/common/SMO.h
  Hardware/Nao/common/NaoCamera.hpp
  Hardware/Nao/common/NaoAudio.hpp
  ${TUHHSDK_HEADERS}
)

set(NAO5_SOURCES
  Hardware/Nao/v5/NaoInterface.cpp
  Hardware/Nao/v5/Nao5Camera.cpp
  ${NAO_SOURCES}
)

set(NAO5_HEADERS
  Hardware/Nao/v5/NaoInterface.hpp
  Hardware/Nao/v5/Nao5Camera.hpp
  ${NAO_HEADERS}
)

set(NAO6_SOURCES
  Hardware/Nao/v6/Nao6Interface.cpp
  Hardware/Nao/v6/Nao6Camera.cpp
  ${NAO_SOURCES}
)

set(NAO6_HEADERS
  Hardware/Nao/v6/Nao6Interface.hpp
  Hardware/Nao/v6/Nao6Camera.hpp
  ${NAO_HEADERS}
)

set(QT_WEBSOCKET_SOURCES
  Hardware/QtWebsocket/main.cpp
  Hardware/QtWebsocket/QtWebsocketCamera.cpp
  Hardware/QtWebsocket/QtWebsocketInterface.cpp
  ${TUHHSDK_SOURCES}
)

set(QT_WEBSOCKET_HEADERS
  Hardware/QtWebsocket/QtWebsocketCamera.hpp
  Hardware/QtWebsocket/QtWebsocketInterface.hpp
  ${TUHHSDK_HEADERS}
)

set(REPLAY_SOURCES
  Hardware/Replay/lodepng.cpp
  Hardware/Replay/main.cpp
  Hardware/Replay/ReplayAudio.cpp
  Hardware/Replay/ReplayCamera.cpp
  Hardware/Replay/ReplayFakeData.cpp
  Hardware/Replay/ReplayInterface.cpp
  ${TUHHSDK_SOURCES}
)

set(REPLAY_HEADERS
  Hardware/Replay/lodepng.h
  Hardware/Replay/ReplayAudio.hpp
  Hardware/Replay/ReplayCamera.hpp
  Hardware/Replay/ReplayFakeData.hpp
  Hardware/Replay/ReplayInterface.hpp
  ${TUHHSDK_HEADERS}
)

set(SIMROBOT_SOURCES
  Hardware/SimRobot/HULKsMenu.cpp
  Hardware/SimRobot/main.cpp
  Hardware/SimRobot/SimRobotAdapter.cpp
  Hardware/SimRobot/SimRobotAdapterAdapter.cpp
  Hardware/SimRobot/SimRobotCamera.cpp
  Hardware/SimRobot/SimRobotFakeData.cpp
  Hardware/SimRobot/SimRobotInterface.cpp
  Hardware/SimRobot/SimRobotNoAudio.cpp
  Hardware/SimRobot/SimRobotPortAudio.cpp
  ${TUHHSDK_SOURCES}
)

set(SIMROBOT_HEADERS
  Hardware/SimRobot/HULKsMenu.hpp
  Hardware/SimRobot/SimRobotAdapter.hpp
  Hardware/SimRobot/SimRobotAdapterAdapter.hpp
  Hardware/SimRobot/SimRobotCamera.hpp
  Hardware/SimRobot/SimRobotFakeData.hpp
  Hardware/SimRobot/SimRobotInterface.hpp
  Hardware/SimRobot/SimRobotNoAudio.hpp
  Hardware/SimRobot/SimRobotPortAudio.hpp
  ${TUHHSDK_HEADERS}
)

# Tell local targets where config etc. is located.
if(DEFINED ENV{LOCAL_FILE_ROOT})
  add_definitions(-DLOCAL_FILE_ROOT="$ENV{LOCAL_FILE_ROOT}")
else()
  if(NOT NAO)
    if(WIN32)
      add_definitions(-DLOCAL_FILE_ROOT="${CMAKE_SOURCE_DIR}/home/")
    else(WIN32)
      add_definitions(-DLOCAL_FILE_ROOT="${CMAKE_SOURCE_DIR}/webots/")
    endif(WIN32)
  endif(NOT NAO)
endif()

include_directories(${CMAKE_CURRENT_SOURCE_DIR})

if(NAO)
  if(NAO_V5)
    add_executable(${PROJECT_NAME}Nao ${NAO5_SOURCES} ${NAO5_HEADERS} $<TARGET_OBJECTS:Brain> $<TARGET_OBJECTS:Vision> $<TARGET_OBJECTS:Motion>)
  elseif(NAO_V6)
    add_executable(${PROJECT_NAME}Nao ${NAO6_SOURCES} ${NAO6_HEADERS} $<TARGET_OBJECTS:Brain> $<TARGET_OBJECTS:Vision> $<TARGET_OBJECTS:Motion>)
  endif(NAO_V5)
  target_include_directories(${PROJECT_NAME}Nao SYSTEM PUBLIC ${TUHH_DEPS_INCLUDE_DIRECTORIES})
  target_link_libraries(${PROJECT_NAME}Nao ${TUHH_DEPS_LIBRARIES})

  if(NAOLIB AND NAO_V5)
    # subtarget for tuhhALModule as external project because it needs a
    # different toolchain
    include(ExternalProject)
    ExternalProject_Add(${PROJECT_NAME}ALModule
      SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/Hardware/Nao/v5
      DOWNLOAD_COMMAND ""
      UPDATE_COMMAND ""
      CMAKE_ARGS
        -DCMAKE_BUILD_TYPE=${CMAKE_BUILD_TYPE}
        -DOPTIMIZE_FOR_TARGET=ATOM
        -DTOOLCHAIN_DIR=${TOOLCHAIN_DIR}
      BUILD_ALWAYS 1
      INSTALL_COMMAND
        ${CMAKE_COMMAND} -E copy lib${PROJECT_NAME}ALModule.so ${CMAKE_CURRENT_BINARY_DIR})
    add_dependencies(${PROJECT_NAME}Nao ${PROJECT_NAME}ALModule)
  endif(NAOLIB AND NAO_V5)

  assign_source_group(${NAO_SOURCES} ${NAO_HEADERS})

  if(NOT WIN32)
    if(NAO_V5)
      add_custom_target(postBuildHook ALL
        COMMAND ../../../../../scripts/linkBuild -t nao5 -b ${CMAKE_BUILD_TYPE}
        DEPENDS ${PROJECT_NAME}Nao)
    elseif(NAO_V6)
      add_custom_target(postBuildHook ALL
        COMMAND ../../../../../scripts/linkBuild -t nao6 -b ${CMAKE_BUILD_TYPE}
        DEPENDS ${PROJECT_NAME}Nao)
    endif(NAO_V5)
  endif(NOT WIN32)
endif(NAO)

if(QT_WEBSOCKET)
  # Find includes in corresponding build directories
  set(CMAKE_INCLUDE_CURRENT_DIR ON)
  # Instruct CMake to run moc automatically when needed.
  set(CMAKE_AUTOMOC ON)

  add_executable(${PROJECT_NAME}QtWebsocket ${QT_WEBSOCKET_SOURCES} ${QT_WEBSOCKET_HEADERS} $<TARGET_OBJECTS:Brain> $<TARGET_OBJECTS:Vision> $<TARGET_OBJECTS:Motion>)
  target_include_directories(${PROJECT_NAME}QtWebsocket SYSTEM PUBLIC ${TUHH_DEPS_INCLUDE_DIRECTORIES})
  target_link_libraries(${PROJECT_NAME}QtWebsocket ${TUHH_DEPS_LIBRARIES})
  #create library for simulations

  find_package(Qt5Core REQUIRED)
  target_include_directories(${PROJECT_NAME}QtWebsocket SYSTEM PUBLIC ${Qt5Core_INCLUDE_DIRS})
  target_link_libraries(${PROJECT_NAME}QtWebsocket ${Qt5Core_LIBRARIES} ${Qt5Core_QTMAIN_LIBRARIES})

  find_package(Qt5Network REQUIRED)
  target_include_directories(${PROJECT_NAME}QtWebsocket SYSTEM PUBLIC ${Qt5Network_INCLUDE_DIRS})
  target_link_libraries(${PROJECT_NAME}QtWebsocket ${Qt5Network_LIBRARIES})

  find_package(Qt5WebSockets REQUIRED)
  target_include_directories(${PROJECT_NAME}QtWebsocket SYSTEM PUBLIC ${Qt5WebSockets_INCLUDE_DIRS})
  target_link_libraries(${PROJECT_NAME}QtWebsocket ${Qt5WebSockets_LIBRARIES})

  assign_source_group(${QT_WEBSOCKET_SOURCES} ${QT_WEBSOCKET_HEADERS})
endif(QT_WEBSOCKET)

if(SIMROBOT)
  set(CMAKE_INCLUDE_CURRENT_DIR ON)
  set(CMAKE_AUTOMOC ON)
  if(WIN32)
    add_definitions("-D WINDOWS")
  endif(WIN32)

  add_library(${PROJECT_NAME}SimRobot SHARED ${SIMROBOT_SOURCES} ${SIMROBOT_HEADERS} $<TARGET_OBJECTS:Brain> $<TARGET_OBJECTS:Vision> $<TARGET_OBJECTS:Motion>)
  target_include_directories(${PROJECT_NAME}SimRobot SYSTEM PUBLIC ${TUHH_DEPS_INCLUDE_DIRECTORIES})
  target_link_libraries(${PROJECT_NAME}SimRobot ${TUHH_DEPS_LIBRARIES})

  find_package(Qt5Core REQUIRED)
  target_link_libraries(${PROJECT_NAME}SimRobot Qt5::Core)

  find_package(Qt5Widgets REQUIRED)
  target_link_libraries(${PROJECT_NAME}SimRobot Qt5::Widgets)

  target_include_directories(${PROJECT_NAME}SimRobot SYSTEM PUBLIC ${SIMROBOT_INCLUDE_DIRS})

  assign_source_group(${SIMROBOT_SOURCES} ${SIMROBOT_HEADERS})

  if(NOT WIN32)
    add_custom_target(postBuildHook ALL
      COMMAND ../../../../../scripts/linkBuild -t simrobot -b ${CMAKE_BUILD_TYPE}
      DEPENDS ${PROJECT_NAME}SimRobot)
  endif(NOT WIN32)

endif(SIMROBOT)

if(REPLAY)
  add_executable(${PROJECT_NAME}Replay ${REPLAY_SOURCES} ${REPLAY_HEADERS} $<TARGET_OBJECTS:Brain> $<TARGET_OBJECTS:Vision> $<TARGET_OBJECTS:Motion>)
  target_include_directories(${PROJECT_NAME}Replay SYSTEM PUBLIC ${TUHH_DEPS_INCLUDE_DIRECTORIES})
  target_link_libraries(${PROJECT_NAME}Replay ${TUHH_DEPS_LIBRARIES})

  assign_source_group(${REPLAY_SOURCES} ${REPLAY_HEADERS})

  if(NOT WIN32)
    add_custom_target(postBuildHook ALL
      COMMAND ../../../../../scripts/linkBuild -t replay -b ${CMAKE_BUILD_TYPE}
      DEPENDS ${PROJECT_NAME}Replay)
  endif(NOT WIN32)
endif(REPLAY)
