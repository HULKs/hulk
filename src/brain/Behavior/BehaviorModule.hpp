#pragma once

#include <mutex>

#include "Data/BallSearchPosition.hpp"
#include "Data/BallState.hpp"
#include "Data/BishopPosition.hpp"
#include "Data/BodyPose.hpp"
#include "Data/ButtonData.hpp"
#include "Data/SetPlayStrikerAction.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/DefenderAction.hpp"
#include "Data/DefendingPosition.hpp"
#include "Data/EyeLEDRequest.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/HeadMotionOutput.hpp"
#include "Data/HeadPositionData.hpp"
#include "Data/KeeperAction.hpp"
#include "Data/MotionRequest.hpp"
#include "Data/MotionState.hpp"
#include "Data/PenaltyKeeperAction.hpp"
#include "Data/PenaltyStrikerAction.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/PointOfInterests.hpp"
#include "Data/ReplacementKeeperAction.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/SetPosition.hpp"
#include "Data/SitDownOutput.hpp"
#include "Data/StrikerAction.hpp"
#include "Data/SupportingPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Data/WorldState.hpp"
#include "Framework/Module.hpp"

#include "DataSet.hpp"


class Brain;

class BehaviorModule : public Module<BehaviorModule, Brain>
{
public:
  /// the name of this module
  ModuleName name = "BehaviorModule";
  /**
   * @brief BehaviorModule initializes members
   * @param manager a reference to brain
   */
  BehaviorModule(const ModuleManagerInterface& manager);
  /**
   * @brief cycle executes the behavior
   */
  void cycle();

private:
  /// mutex that locks the actual remote motion request
  std::mutex actualRemoteMotionRequestLock_;
  /// the remote motion request (may be changed by other threads)
  Parameter<MotionRequest> remoteMotionRequest_;
  /// whether the remote motion request shall be used (can only be activated in INITIAL)
  Parameter<bool> useRemoteMotionRequest_;
  /// the game controller state
  const Dependency<GameControllerState> gameControllerState_;
  /// the ball state
  const Dependency<BallState> ballState_;
  /// the robot position
  const Dependency<RobotPosition> robotPosition_;
  /// the body pose
  const Dependency<BodyPose> bodyPose_;
  /// the player configuration
  const Dependency<PlayerConfiguration> playerConfiguration_;
  /// the player roles
  const Dependency<PlayingRoles> playingRoles_;
  /// the motion state
  const Dependency<MotionState> motionState_;
  /// the head command data
  const Dependency<HeadPositionData> headPositionData_;
  /// the head motion output
  const Dependency<HeadMotionOutput> headMotionOutput_;
  /// the sit down output
  const Dependency<SitDownOutput> sitDownOutput_;
  /// the team ball model
  const Dependency<TeamBallModel> teamBallModel_;
  /// my homies
  const Dependency<TeamPlayers> teamPlayers_;
  /// the ball search position
  const Dependency<BallSearchPosition> ballSearchPosition_;
  /// the field dimensions
  const Dependency<FieldDimensions> fieldDimensions_;
  /// the striker action
  const Dependency<StrikerAction> strikerAction_;
  /// the penalty striker action
  const Dependency<PenaltyStrikerAction> penaltyStrikerAction_;
  /// the penalty striker action
  const Dependency<SetPlayStrikerAction> setPlayStrikerAction_;
  /// the kick configuration
  const Dependency<KickConfigurationData> kickConfigurationData_;
  /// the keeper action
  const Dependency<KeeperAction> keeperAction_;
  /// the penalty keeper action
  const Dependency<PenaltyKeeperAction> penaltyKeeperAction_;
  /// the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// the set position
  const Dependency<SetPosition> setPosition_;
  /// the defender action
  const Dependency<DefenderAction> defenderAction_;
  /// the defending position
  const Dependency<DefendingPosition> defendingPosition_;
  /// the bishop position
  const Dependency<BishopPosition> bishopPosition_;
  /// the supporting position
  const Dependency<SupportingPosition> supportingPosition_;
  /// the replacement keeper action
  const Dependency<ReplacementKeeperAction> replacementKeeperAction_;
  /// the point of interests
  const Dependency<PointOfInterests> pointOfInterests_;
  /// the button data
  const Dependency<ButtonData> buttonData_;
  /// the world state
  const Dependency<WorldState> worldState_;
  /// the motion request
  Production<MotionRequest> motionRequest_;
  /// the eye LED request
  Production<EyeLEDRequest> eyeLEDRequest_;
  /// the last action command that was computed by the behavior
  ActionCommand actionCommand_;
  /// the data set/bundle that is passed to the behavior
  DataSet dataSet_;
  /// a thread-safe copy of the remote motion request
  MotionRequest actualRemoteMotionRequest_;
};
