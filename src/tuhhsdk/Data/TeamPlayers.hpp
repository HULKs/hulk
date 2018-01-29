#pragma once

#include <list>

#include "Data/PlayingRoles.hpp"
#include "Definitions/BHULKsStandardMessage.h"
#include "Framework/DataType.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Time.hpp"

// These values are taken from the SPLStandardMessage and must have exactly the values that are specified there.
enum class PlayerIntention
{
  NOTHING = 0,
  KEEPER = 1,
  DEFENDER = 2,
  PLAY_BALL = 3,
  LOST = 4
};

enum class PlayerSuggestion
{
  NOTHING = 0,
  KEEPER = 1,
  DEFENSE = 2,
  OFFENSE = 3,
  PLAY_BALL = 4
};

struct TeamPlayer : public Uni::To, public Uni::From
{
  /// time (seconds) since the message from the robot has been received
  float age;
  /// the number of the player
  unsigned int playerNumber;
  /// flag to distinguish HULK robots from other team members
  bool isHULK;
  /// the pose on the field (meters, radians)
  Pose pose;
  /// describes the current confidence of a robot about its self-location (0..100)
  uint8_t currentSideConfidence;
  /// describes the current confidence of a robot about playing in the right direction (0..100)
  uint8_t currentPositionConfidence;
  /// the position where the robots wants to be (meters)
  Vector2f target;
  /// the position (relative to the sending robot) of the ball (meters)
  Vector2f ballPosition;
  /// the relative velocity of the ball (meters per second)
  Vector2f ballVelocity;
  /// time when the robot has seen the ball
  TimePoint timeWhenBallWasSeen;
  /// the obstacles this robot reported (from his local obstacle model) - distances in meters!
  std::vector<B_HULKs::Obstacle> localObstacles;
  /// whether the robot is fallen
  bool fallen;
  /// whether the robot is penalized
  bool penalized;
  /// if keeper wants to play ball
  bool keeperWantsToPlayBall;
  /// what the robot wants to do
  PlayerIntention intention;
  /// what the robot wants me to do
  PlayerSuggestion suggestion;
  /// the role the player currently performs
  PlayingRole currentlyPerfomingRole;
  /// the roles this player would asign to everyone
  std::vector<PlayingRole> roleAssignments;
  /// the yaw angle of this NAO's head (in rad)
  float headYaw;
  /// the estimated time when this mate would reach the ball
  TimePoint timeWhenReachBall;
  /// the estimated time when this mate would reach the ball as striker
  TimePoint timeWhenReachBallStriker;
  /// the last timestamp when the whistle has been heard
  TimePoint lastTimeWhistleHeard;
  /// the current player number to which this mate want to pass, 0 if no pass target.
  int currentPassTarget;
  /// the timestamp of the last time the robot pose jumped significantly (discontinuity in pose estimation)
  TimePoint timestampLastJumped;
  /// the position the robot is currently exploring
  Vector2f currentSearchPosition;
  /// the positions the robot is currently suggesting. (Index + 1 ^= search position for robot with player number Index + 1).
  VecVector2f suggestedSearchPositions;

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["age"] << age;
    value["playerNumber"] << playerNumber;
    value["isHULK"] << isHULK;
    value["pose"] << pose;
    value["currentSideConfidence"] << currentSideConfidence;
    value["currentPositionConfidence"] << currentPositionConfidence;
    value["target"] << target;
    value["ballPosition"] << ballPosition;
    value["ballVelocity"] << ballVelocity;
    value["timeWhenBallWasSeen"] << timeWhenBallWasSeen;
    value["fallen"] << fallen;
    value["penalized"] << penalized;
    value["keeperWantsToPlayBall"] << keeperWantsToPlayBall;
    value["intention"] << static_cast<int>(intention);
    value["suggestion"] << static_cast<int>(suggestion);
    value["currentlyPerfomingRole"] << currentlyPerfomingRole;
    value["roleAssignments"] << roleAssignments;
    value["headYaw"] << headYaw;
    value["timeWhenReachBall"] << timeWhenReachBall;
    value["timeWhenReachBallStriker"] << timeWhenReachBallStriker;
    value["lastTimeWhistleHeard"] << lastTimeWhistleHeard;
    value["currentPassTarget"] << currentPassTarget;
    value["timestampLastJumped"] << timestampLastJumped;
    value["currentSearchPosition"] << currentSearchPosition;
    value["suggestedSearchPositions"] << suggestedSearchPositions;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    int valueRead;

    value["age"] >> age;
    value["playerNumber"] >> playerNumber;
    value["isHULK"] >> isHULK;
    value["pose"] >> pose;
    value["currentSideConfidence"] >> valueRead;
    currentSideConfidence = static_cast<uint8_t>(valueRead);
    value["currentPositionConfidence"] >> valueRead;
    currentPositionConfidence = static_cast<uint8_t>(valueRead);
    value["target"] >> target;
    value["ballPosition"] >> ballPosition;
    value["ballVelocity"] >> ballVelocity;
    value["timeWhenBallWasSeen"] >> timeWhenBallWasSeen;
    value["fallen"] >> fallen;
    value["penalized"] >> penalized;
    value["keeperWantsToPlayBall"] >> keeperWantsToPlayBall;
    value["intention"] >> valueRead;
    intention = static_cast<PlayerIntention>(valueRead);
    value["suggestion"] >> valueRead;
    suggestion = static_cast<PlayerSuggestion>(valueRead);
    value["currentlyPerfomingRole"] >> valueRead;
    value["roleAssignments"] >> roleAssignments;
    value["headYaw"] >> headYaw;
    value["timeWhenReachBall"] >> timeWhenReachBall;
    value["timeWhenReachBallStriker"] >> timeWhenReachBallStriker;
    value["lastTimeWhistleHeard"] >> lastTimeWhistleHeard;
    value["currentPassTarget"] >> currentPassTarget;
    value["timestampLastJumped"] >> timestampLastJumped;
    value["currentSearchPosition"] >> currentSearchPosition;
    value["suggestedSearchPositions"] >> suggestedSearchPositions;
  }
};

class TeamPlayers : public DataType<TeamPlayers>
{
public:
  /// list of the teammates that have sent messages recently
  std::list<TeamPlayer> players;
  /// the number of active (i.e. unpenalized) players
  unsigned int activePlayers;
  /// the number of active (i.e. unpenalized) players not including other players (during mixed team challenge)
  unsigned int activeHULKPlayers;
  /**
   * @brief reset clears the list of players
   */
  void reset()
  {
    activePlayers = 0;
    activeHULKPlayers = 0;
    players.clear();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["players"] << players;
    value["activePlayers"] << activePlayers;
    value["activeHULKPlayers"] << activeHULKPlayers;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["players"] >> players;
    value["activePlayers"] >> activePlayers;
    value["activeHULKPlayers"] >> activeHULKPlayers;
  }
};
