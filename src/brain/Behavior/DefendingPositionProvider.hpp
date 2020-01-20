#pragma once

#include "Data/DefendingPosition.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/ObstacleData.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Data/WorldState.hpp"
#include "Framework/Module.hpp"


class Brain;

class DefendingPositionProvider : public Module<DefendingPositionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "DefendingPositionProvider";
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
   * @brief findRelevantTeamPlayers searches the team players for a support striker and the other
   * defender
   * @param keeper the team player which is currently the keeper (if there is any)
   * @param replacementKeeper the team player which is currently the replacement keeper (if there is
   * any)
   * @param supportStriker the team player which is currently support striker (if there is any)
   * @param otherDefender the team player which is the other defender (if there is any)
   */
  void findRelevantTeamPlayers(const TeamPlayer*& keeper, const TeamPlayer*& replacementKeeper,
                               const TeamPlayer*& supportStriker,
                               const TeamPlayer*& otherDefender) const;

  /**
   * @brief considerSetPlay checks if the enemy team has a free kick and adjusts the
   * defender position if it is not legal.
   */
  void considerSetPlay();

  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<ObstacleData> obstacleData_;
  const Dependency<PlayingRoles> playingRoles_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<TeamPlayers> teamPlayers_;
  const Dependency<WorldState> worldState_;
  Production<DefendingPosition> defendingPosition_;
  /*
   * The defending positions are based on three defense lines parallel to the base line.
   * Based on the ball position, the other defender, and the existence of the supporter different
   * positions are chosen.
   */
  const float passiveDefenseLineX_;
  const float neutralDefenseLineX_;
  const float aggressiveDefenseLineX_;
  const float passiveDefenseLineY_;
  /// whether I am far aways from my own goal
  bool iAmFar_;
  /// whether the other defender is far away from our own goal
  bool otherIsFar_;
  /// whether the ball is close to our own goal
  bool ballCloseToOwnGoal_;
  /// hysteresis for ball and robot position decisions
  const float hysteresis_ = 0.25f;
};
