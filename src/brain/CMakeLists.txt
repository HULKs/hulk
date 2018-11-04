cmake_minimum_required(VERSION 3.5)
project(Brain)

#set sources of your project here
set(HEADERS
  print.h
  Behavior/ActionCommand.hpp
  Behavior/BehaviorModule.hpp
  Behavior/BehaviorParameters.hpp
  Behavior/BishopPositionProvider.hpp
  Behavior/DefendingPositionProvider.hpp
  Behavior/DataSet.hpp
  Behavior/BallSearchMapManager.hpp
  Behavior/HeadPositionProvider.hpp
  Behavior/BallSearchPositionProvider.hpp
  Behavior/KeeperActionProvider.hpp
  Behavior/PenaltyKeeperActionProvider.hpp
  Behavior/PlayingRoleProvider.hpp
  Behavior/ReplacementKeeperActionProvider.hpp
  Behavior/SetPositionProvider.hpp
  Behavior/StrikerActionProvider.hpp
  Behavior/PenaltyStrikerActionProvider.hpp
  Behavior/SupportingPositionProvider.hpp
  Behavior/TimeToReachBallProvider.hpp
  Behavior/Units.hpp
  Behavior/BehaviorUnits/NotPenalized.hpp
  Behavior/BehaviorUnits/RootBehavior.hpp
  Behavior/BehaviorUnits/GameStates/Finished.hpp
  Behavior/BehaviorUnits/GameStates/Initial.hpp
  Behavior/BehaviorUnits/GameStates/PenaltyShootout.hpp
  Behavior/BehaviorUnits/GameStates/Playing.hpp
  Behavior/BehaviorUnits/GameStates/Ready.hpp
  Behavior/BehaviorUnits/GameStates/Set.hpp
  Behavior/BehaviorUnits/Head/CameraCalibration.hpp
  Behavior/BehaviorUnits/Head/ActiveVision.hpp
  Behavior/BehaviorUnits/Head/LookAround.hpp
  Behavior/BehaviorUnits/Head/LookForward.hpp
  Behavior/BehaviorUnits/Head/TrackBall.hpp
  Behavior/BehaviorUnits/Roles/Bishop.hpp
  Behavior/BehaviorUnits/Roles/Defender.hpp
  Behavior/BehaviorUnits/Roles/Demo.hpp
  Behavior/BehaviorUnits/Roles/Keeper.hpp
  Behavior/BehaviorUnits/Roles/ShootOnHeadTouch.hpp
  Behavior/BehaviorUnits/Roles/Striker.hpp
  Behavior/BehaviorUnits/Roles/SupportStriker.hpp
  Behavior/BehaviorUnits/Skills/Dribble.hpp
  Behavior/BehaviorUnits/Skills/Kick.hpp
  Behavior/BehaviorUnits/Skills/InWalkKick.hpp
  Behavior/BehaviorUnits/Skills/Rotate.hpp
  Behavior/BehaviorUnits/Skills/SearchForBall.hpp
  Behavior/BehaviorUnits/Skills/StandUp.hpp
  Behavior/BehaviorUnits/Skills/WalkBehindBall.hpp
  Behavior/BehaviorUnits/Skills/WalkBehindBallAndDribble.hpp
  Behavior/BehaviorUnits/Skills/WalkToPose.hpp
  Behavior/WorldStateProvider.hpp
  Interface/AudioReceiver.hpp
  Interface/AudioPlayer.hpp
  GameController/GameController.hpp
  GameController/GameControllerModifier.hpp
  Knowledge/Ball/BallFilter.hpp
  Knowledge/Ball/FakeBallProvider.hpp
  Knowledge/Ball/TeamBallFilter.hpp
  Knowledge/LandmarkFilter/LandmarkFilter.hpp
  Knowledge/Position/FakeRobotPoseProvider.hpp
  Knowledge/Position/UKFPositionKnowledge.hpp
  Knowledge/Position/ParticlePositionKnowledge.hpp
  Knowledge/Position/PositionParticle.hpp
  Knowledge/Position/PositionProvider.hpp
  Knowledge/Position/PoseHypothesis.hpp
  Knowledge/Position/UKFPose2D.hpp
  Knowledge/Position/FieldInfo.hpp
  Knowledge/Obstacles/TeamObstacleFilter.hpp
  Knowledge/Obstacles/ObstacleFilter.hpp
  Modules/CollisionDetection/CollisionDetector.hpp
  Modules/FootCollisionDetector/FootCollisionDetector.hpp
  Modules/HeadDamageProvider/HeadDamageProvider.hpp
  Modules/MotionPlanning/MotionPlanner.hpp
  Modules/WhistleDetection/WhistleDetection.hpp
  Modules/ReplayRecorder/ReplayRecorder.hpp
  Modules/OdometryOffsetProvider/OdometryOffsetProvider.hpp
  Modules/TeamPlayersAugmenter/TeamPlayersAugmenter.hpp
  SPLNetwork/BHULKsHelper.hpp
  SPLNetwork/HULKsMessage.hpp
  SPLNetwork/SPLNetworkService.hpp
  SPLNetwork/SPLMessageReceiver.hpp
  SPLNetwork/SPLMessageTransmitter.hpp
  Brain.hpp
  BrainThread.hpp
)

set(SOURCES
  Behavior/BallSearchMapManager.cpp
  Behavior/BallSearchPositionProvider.cpp
  Behavior/BehaviorModule.cpp
  Behavior/BishopPositionProvider.cpp
  Behavior/DefendingPositionProvider.cpp
  Behavior/HeadPositionProvider.cpp
  Behavior/KeeperActionProvider.cpp
  Behavior/PenaltyKeeperActionProvider.cpp
  Behavior/PlayingRoleProvider.cpp
  Behavior/ReplacementKeeperActionProvider.cpp
  Behavior/SetPositionProvider.cpp
  Behavior/StrikerActionProvider.cpp
  Behavior/PenaltyStrikerActionProvider.cpp
  Behavior/SupportingPositionProvider.cpp
  Behavior/TimeToReachBallProvider.cpp
  Behavior/WorldStateProvider.cpp
  Interface/AudioReceiver.cpp
  Interface/AudioPlayer.cpp
  GameController/GameController.cpp
  GameController/GameControllerModifier.cpp
  Knowledge/Ball/BallFilter.cpp
  Knowledge/Ball/FakeBallProvider.cpp
  Knowledge/Ball/TeamBallFilter.cpp
  Knowledge/LandmarkFilter/LandmarkFilter.cpp
  Knowledge/Position/FakeRobotPoseProvider.cpp
  Knowledge/Position/UKFPositionKnowledge.cpp
  Knowledge/Position/ParticlePositionKnowledge.cpp
  Knowledge/Position/PositionProvider.cpp
  Knowledge/Position/PoseHypothesis.cpp
  Knowledge/Position/UKFPose2D.cpp
  Knowledge/Position/FieldInfo.cpp
  Knowledge/Obstacles/TeamObstacleFilter.cpp
  Knowledge/Obstacles/ObstacleFilter.cpp
  Modules/CollisionDetection/CollisionDetector.cpp
  Modules/FootCollisionDetector/FootCollisionDetector.cpp
  Modules/HeadDamageProvider/HeadDamageProvider.cpp
  Modules/MotionPlanning/MotionPlanner.cpp
  Modules/WhistleDetection/WhistleDetection.cpp
  Modules/ReplayRecorder/ReplayRecorder.cpp
  Modules/OdometryOffsetProvider/OdometryOffsetProvider.cpp
  Modules/TeamPlayersAugmenter/TeamPlayersAugmenter.cpp
  SPLNetwork/HULKsMessage.cpp
  SPLNetwork/SPLNetworkService.cpp
  SPLNetwork/SPLMessageReceiver.cpp
  SPLNetwork/SPLMessageTransmitter.cpp
  Brain.cpp
  BrainThread.cpp
  print.cpp
)


add_library(${PROJECT_NAME} OBJECT ${SOURCES} ${HEADERS})
target_include_directories(${PROJECT_NAME} PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

find_package(tuhhSDK REQUIRED)
target_include_directories(${PROJECT_NAME} PUBLIC ${TUHHSDK_INCLUDE_DIRS})

target_include_directories(${PROJECT_NAME} SYSTEM PUBLIC ${TUHH_DEPS_INCLUDE_DIRECTORIES})

assign_source_group(${SOURCES} ${HEADERS})
