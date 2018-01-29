#pragma once

#include "Data/BallState.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/KeeperAction.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Framework/Module.hpp"

class Brain;

class KeeperActionProvider : public Module<KeeperActionProvider, Brain>
{
public:
  /**
   * @brief KeeperActionProvider initializes members
   * @param manager a reference to brain
   */
  KeeperActionProvider(const ModuleManagerInterface& manager);
  /**
   * @brief cycle updates the Keeper action
   */
  void cycle();

private:
  /// whether genuflect is an option
  const Parameter<bool> mayGenuflect_;
  /// a reference to the team ball state
  const Dependency<TeamBallModel> teamBallModel_;
  /// a reference to the ball state
  const Dependency<BallState> ballState_;
  /// a reference to the field dimensions
  const Dependency<FieldDimensions> fieldDimensions_;
  /// a reference to the robot position
  const Dependency<RobotPosition> robotPosition_;
  /// a reference to the team players
  const Dependency<TeamPlayers> teamPlayers_;
  /// a reference to the game controller state
  const Dependency<GameControllerState> gameControllerState_;

  /// a reference to the striker action
  Production<KeeperAction> keeperAction_;

  KeeperAction::Type lastAction_;
  Vector2f keeperPosition_;

  bool shouldGenuflect() const;
};
