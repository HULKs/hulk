#pragma once

#include "Data/DefendingPosition.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Data/WorldState.hpp"
#include "Framework/Module.hpp"
#include "Tools/Math/Eigen.hpp"


class Brain;

class DefendingPositionProvider : public Module<DefendingPositionProvider, Brain>
{
public:
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
   * @brief findRelevantTeamPlayers searches the team players for a keeper and the other defender
   * @param keeper the team player which is currently keeper (if there is any)
   * @param otherDefender the team player which is the other defender (if there is any)
   */
  void findRelevantTeamPlayers(const TeamPlayer*& keeper, const TeamPlayer*& otherDefender) const;

  /**
   * @brief getCirclePosition determines a position on a circle centered in the own goal
   * @param radius the radius of the circle
   * @param left whether the position shall be a left or a right position
   */
  Vector2f getCirclePosition(const float radius, const bool left);
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<PlayingRoles> playingRoles_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<TeamPlayers> teamPlayers_;
  const Dependency<WorldState> worldState_;
  Production<DefendingPosition> defendingPosition_;
  /// the center of the goal
  const Vector2f goalCenter_;
  /// the minimum radius of a circle from the center of the own goal
  const float minRadius_;
};
