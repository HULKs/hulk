#pragma once

#include "Data/DefendingPosition.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/WorldState.hpp"
#include "Framework/Module.hpp"


class Brain;

class DefendingPositionProvider : public Module<DefendingPositionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"DefendingPositionProvider"};
  /**
   * @brief DefendingPositionProvider initializes members
   * @param manager a reference to brain
   */
  DefendingPositionProvider(const ModuleManagerInterface& manager);
  /**
   * @brief cycle calculates the defending position
   */
  void cycle();

private:
  /**
   * @brief calculates the defending position without considering the set Play state
   */
  void calculateDefendingPosition();

  /**
   * @brief considerSetPlay checks if the enemy team has a free kick and adjusts the
   * defender position if it is not legal.
   */
  void considerSetPlay();

  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<PlayingRoles> playingRoles_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<WorldState> worldState_;
  Production<DefendingPosition> defendingPosition_;

  const float passiveDefenseLineX_;
  const float passiveDefenseLineY_;
};
