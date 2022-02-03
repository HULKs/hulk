#pragma once

#include "Data/BallState.hpp"
#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Data/TimeToReachBall.hpp"
#include "Data/WalkGeneratorOutput.hpp"
#include "Data/WorldState.hpp"
#include "Framework/Module.hpp"
#include "Hardware/Clock.hpp"
#include <chrono>
#include <limits>
#include <string>


class Brain;

class PlayingRoleProvider : public Module<PlayingRoleProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"PlayingRoleProvider"};
  explicit PlayingRoleProvider(const ModuleManagerInterface& manager);
  void cycle() override;

private:
  struct Player
  {
    Player(const unsigned int playerNumber, const Vector2f& pos)
      : playerNumber(playerNumber)
      , position(pos)
    {
    }
    unsigned int playerNumber;
    Vector2f position;
  };

  enum class BallSearchState
  {
    /// not in ball search
    NONE,
    /// short term ball search with defender and loser role
    SHORT_TERM,
    /// long term ball search without defender or loser role
    LONG_TERM
  };

  const Parameter<bool> useTeamRole_;
  const Parameter<bool> assignBishop_;
  const Parameter<bool> assignBishopWithLessThanFourFieldPlayers_;
  const Parameter<bool> playerOneCanBecomeStriker_;
  const Parameter<float> playerOneDistanceThreshold_;
  const Parameter<Clock::duration> keeperTimeToReachBallPenalty_;
  const Parameter<float> keeperInGoalDistanceThreshold_;
  const Parameter<bool> strikeOwnBall_;
  const Parameter<bool> allowFastRoleOverride_;
  const Parameter<Clock::duration> maxFastRoleOverrideDuration_;
  const Parameter<std::string> forceRole_;
  const Parameter<Clock::duration> shortTermBallSearchDuration_;
  const Parameter<Clock::duration> loserDuration_;
  const Dependency<BallState> ballState_;
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<PlayerConfiguration> playerConfiguration_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<TeamPlayers> teamPlayers_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<BodyPose> bodyPose_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<TimeToReachBall> timeToReachBall_;
  const Dependency<WalkGeneratorOutput> walkGeneratorOutput_;
  const Dependency<WorldState> worldState_;

  Production<PlayingRoles> playingRoles_;

  std::vector<PlayingRole> lastAssignment_;

  /// whether we are currently disregarding the role assigment of other players.
  bool revolting_;

  /// the time we last started revolting
  Clock::time_point startOfLastRevolution_;

  /// the current state of the ball search
  BallSearchState ballSearchState_{BallSearchState::NONE};

  /// whether loser has been assigned
  bool loserAssigned_{false};

  /// player number of the last striker
  unsigned int lastStrikerNumber_{0};

  /// whether the robot with player number one is far away from own goal
  bool playerOneWasFarAway_;

  /// whether we are currently near the goal
  bool inGoal_{false};

  /**
   * @brief assign the striker role
   */
  void assignStriker();

  /**
   * @brief assign the loser role if necessary
   * @return whether a loser was assigned
   */
  bool assignLoser();

  /**
   * @brief assign the keeper role
   * @return whether a keeper was assigned
   */

  bool assignKeeper();

  /**
   * @brief assign the replacement keeper role
   */
  void assignReplacementKeeper();

  /**
   * @brief getDistanceToGoal returns the distance from a given position to the own goal
   *
   * The distance that is being returned includes a bonus for the player with the player number 1.
   *
   * @param position the position to get the distance for
   * @param playerNumber the player number to get the distance for
   * @return the distance to the own goal including a bonus for player number 1.
   */
  float getDistanceToGoal(const Vector2f& position, const unsigned int playerNumber) const;

  /**
   * @brief whether robot with player number one is far away form our own goal
   * @return true if robot with player number one is far away from our own goal
   */
  bool playerOneIsFarAway();

  /**
   * @brief Assign remaining players to other roles
   */
  void assignRemainingPlayerRoles();

  void assignDefenders(const Player& firstPlayer, const Player& secondPlayer);

  /**
   * @brief set role of player with given number
   * @param playerNumber the number of the players of whom the role is to be set
   * @param role the role to be set
   */
  void updateRole(const unsigned int playerNumber, const PlayingRole role);

  /**
   * @brief convert role string to role enum
   * @param configRole the role string to be converted
   */
  PlayingRole toRole(const std::string& configRole) const;

  /**
   * @brief get actual time to reach ball for given player number
   * @param player number the number of the player of whom the actual time to reach ball is to be
   * determined
   * @param timeToReachBall the duration to reach ball
   * @param timeToReachBallStriker the duration to reach ball if player was striker
   */
  Clock::duration actualTimeToReachBall(unsigned int playerNumber,
                                        const Clock::duration& timeToReachBall,
                                        const Clock::duration& timeToReachBallStriker);
  /**
   * @brief get last role for given player number
   * @param playerNumber the number of the player of whom the role is to be determined
   */
  PlayingRole lastRoleOf(const unsigned int playerNumber) const;
};
