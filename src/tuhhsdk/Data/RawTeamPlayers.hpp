#pragma once

#include <list>

#include "Definitions/RoboCupGameControlData.h"
#include "Data/PlayingRoles.hpp"
#include "Framework/DataType.hpp"
#include "Network/SPLNetwork/HULKsMessage.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Time.hpp"


struct RawTeamPlayer : public Uni::To, public Uni::From
{
  /**
   * @brief RawTeamPlayer initializes members
   */
  RawTeamPlayer()
  {
    for (uint8_t playerIndex = 0; playerIndex <= MAX_NUM_PLAYERS; playerIndex++)
    {
      suggestedSearchPositions[playerIndex] = Vector2f::Zero();
      suggestedSearchPositionsValidity[playerIndex] = false;
    }
  };

  /// time (seconds) since the message from the robot has been received
  float age = 1337.f;
  /// the number of the player
  unsigned int playerNumber = 0;
  /// flag to distinguish HULK robots from other team members
  bool isHULK = true;
  /// the pose on the field (meters, radians)
  Pose pose;
  /// if the robot is confident about it's self localization.
  bool isPoseValid = false;
  /// the pose where the robots wants to be (meters, rad)
  Pose walkingTo;
  /// the position (relative to the sending robot) of the ball (meters)
  Vector2f ballPosition = Vector2f::Zero();
  /// the relative velocity of the ball (meters per second)
  Vector2f ballVelocity = Vector2f::Zero();
  /// time when the robot has seen the ball
  TimePoint timeWhenBallWasSeen;
  /// the obstacles this robot reported (from his local obstacle model) - distances in meters!
  std::vector<HULKs::Obstacle> localObstacles;
  /// whether the robot is fallen
  bool fallen = true;
  /// whether the robot is penalized
  bool penalized = true;
  /// the role the player currently performs
  PlayingRole currentlyPerformingRole = PlayingRole::NONE;
  /// the roles this player would asign to everyone
  std::vector<PlayingRole> roleAssignments;
  /// the yaw angle of this NAO's head (in rad)
  float headYaw = 0.f;
  /// the estimated time when this mate would reach the ball
  TimePoint timeWhenReachBall;
  /// the estimated time when this mate would reach the ball as striker
  TimePoint timeWhenReachBallStriker;
  /// the last timestamp when the whistle has been heard
  TimePoint lastTimeWhistleHeard;
  /// the current player number to which this mate want to pass, 0 if no pass target.
  int currentPassTarget = 0;
  /// the timestamp of the last time the robot pose jumped significantly (discontinuity in pose
  /// estimation)
  TimePoint timestampLastJumped;
  /// the timestamp of the last time the map was not reliable due to penalties etc.
  TimePoint timestampBallSearchMapUnreliable;
  /// the position the robot is currently exploring
  Vector2f currentSearchPosition = Vector2f::Zero();
  /// the positions the robot is currently suggesting.
  /// (Index + 1 ^= search position for robot with player number Index + 1).
  std::array<Vector2f, MAX_NUM_PLAYERS> suggestedSearchPositions;
  /// the valid flag for every suggested search position;
  std::array<bool, MAX_NUM_PLAYERS> suggestedSearchPositionsValidity;
  /// if the robot is available for searching for the ball.
  bool isAvailableForBallSearch = false;
  /// player with the oldest, continuously updated map
  unsigned int mostWisePlayerNumber = 0;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["age"] << age;
    value["playerNumber"] << playerNumber;
    value["isHULK"] << isHULK;
    value["pose"] << pose;
    value["isPoseValid"] << isPoseValid;
    value["walkingTo"] << walkingTo;
    value["ballPosition"] << ballPosition;
    value["ballVelocity"] << ballVelocity;
    value["timeWhenBallWasSeen"] << timeWhenBallWasSeen;
    value["localObstacles"] << localObstacles;
    value["fallen"] << fallen;
    value["penalized"] << penalized;
    value["currentlyPerformingRole"] << currentlyPerformingRole;
    value["roleAssignments"] << roleAssignments;
    value["headYaw"] << headYaw;
    value["timeWhenReachBall"] << timeWhenReachBall;
    value["timeWhenReachBallStriker"] << timeWhenReachBallStriker;
    value["lastTimeWhistleHeard"] << lastTimeWhistleHeard;
    value["currentPassTarget"] << currentPassTarget;
    value["timestampLastJumped"] << timestampLastJumped;
    value["timestampBallSearchMapUnreliable"] << timestampBallSearchMapUnreliable;
    value["currentSearchPosition"] << currentSearchPosition;
    value["suggestedSearchPositions"] << suggestedSearchPositions;
    value["suggestedSearchPositionsValidity"] << suggestedSearchPositionsValidity;
    value["isAvailableForBallSearch"] << isAvailableForBallSearch;
    value["mostWisePlayerNumber"] << mostWisePlayerNumber;
  }

  void fromValue(const Uni::Value& value) override
  {
    int valueRead;

    value["age"] >> age;
    value["playerNumber"] >> playerNumber;
    value["isHULK"] >> isHULK;
    value["pose"] >> pose;
    value["isPoseValid"] >> isPoseValid;
    value["walkingTo"] >> walkingTo;
    value["ballPosition"] >> ballPosition;
    value["ballVelocity"] >> ballVelocity;
    value["timeWhenBallWasSeen"] >> timeWhenBallWasSeen;
    value["localObstacles"] >> localObstacles;
    value["fallen"] >> fallen;
    value["penalized"] >> penalized;
    value["currentlyPerformingRole"] >> valueRead;
    currentlyPerformingRole = static_cast<PlayingRole>(valueRead);
    value["roleAssignments"] >> roleAssignments;
    value["headYaw"] >> headYaw;
    value["timeWhenReachBall"] >> timeWhenReachBall;
    value["timeWhenReachBallStriker"] >> timeWhenReachBallStriker;
    value["lastTimeWhistleHeard"] >> lastTimeWhistleHeard;
    value["currentPassTarget"] >> currentPassTarget;
    value["timestampLastJumped"] >> timestampLastJumped;
    value["timestampBallSearchMapUnreliable"] >> timestampBallSearchMapUnreliable;
    value["currentSearchPosition"] >> currentSearchPosition;
    value["suggestedSearchPositions"] >> suggestedSearchPositions;
    value["suggestedSearchPositionsValidity"] >> suggestedSearchPositionsValidity;
    value["isAvailableForBallSearch"] >> isAvailableForBallSearch;
    value["mostWisePlayerNumber"] >> mostWisePlayerNumber;
  }
};

class RawTeamPlayers : public DataType<RawTeamPlayers>
{
public:
  /// the name of this DataType
  DataTypeName name = "RawTeamPlayers";
  /// list of the teammates that have sent messages recently (does not include this robot)
  std::vector<RawTeamPlayer> rawPlayers;
  /// the number of active (i.e. unpenalized) players
  unsigned int activePlayers;
  /// the number of active (i.e. unpenalized) players not including other players (during mixed team
  /// challenge)
  unsigned int activeHULKPlayers;
  /**
   * @brief reset clears the list of players
   */
  void reset() override
  {
    activePlayers = 0;
    activeHULKPlayers = 0;
    rawPlayers.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["rawPlayers"] << rawPlayers;
    value["activePlayers"] << activePlayers;
    value["activeHULKPlayers"] << activeHULKPlayers;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["rawPlayers"] >> rawPlayers;
    value["activePlayers"] >> activePlayers;
    value["activeHULKPlayers"] >> activeHULKPlayers;
  }
};
