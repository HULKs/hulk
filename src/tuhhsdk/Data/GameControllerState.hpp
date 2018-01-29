#pragma once

#include <vector>

#include "Tools/Time.hpp"
#include "Definitions/RoboCupGameControlData.h"
#include "Framework/DataType.hpp"


enum class GameType
{
  ROUNDROBIN = GAME_ROUNDROBIN,
  PLAYOFF = GAME_PLAYOFF,
  MIXEDTEAM_ROUNDROBIN = GAME_MIXEDTEAM_ROUNDROBIN,
  MIXEDTEAM_PLAYOFF = GAME_MIXEDTEAM_PLAYOFF
};

enum class GameState
{
  INITIAL = STATE_INITIAL,
  READY = STATE_READY,
  SET = STATE_SET,
  PLAYING = STATE_PLAYING,
  FINISHED = STATE_FINISHED
};

enum class SecondaryState
{
  NORMAL = STATE2_NORMAL,
  PENALTYSHOOT = STATE2_PENALTYSHOOT,
  OVERTIME = STATE2_OVERTIME,
  TIMEOUT = STATE2_TIMEOUT
};

enum class TeamColor
{
  BLUE = TEAM_BLUE,
  RED = TEAM_RED,
  YELLOW = TEAM_YELLOW,
  BLACK = TEAM_BLACK,
  WHITE = TEAM_WHITE,
  GREEN = TEAM_GREEN,
  ORANGE = TEAM_ORANGE,
  PURPLE = TEAM_PURPLE,
  BROWN = TEAM_BROWN,
  GRAY = TEAM_GRAY
};

enum class Penalty
{
  NONE = PENALTY_NONE,
  ILLEGAL_BALL_CONTACT = PENALTY_SPL_ILLEGAL_BALL_CONTACT,
  PLAYER_PUSHING = PENALTY_SPL_PLAYER_PUSHING,
  ILLEGAL_MOTION_IN_SET = PENALTY_SPL_ILLEGAL_MOTION_IN_SET,
  INACTIVE_PLAYER = PENALTY_SPL_INACTIVE_PLAYER,
  ILLEGAL_DEFENDER = PENALTY_SPL_ILLEGAL_DEFENDER,
  LEAVING_THE_FIELD = PENALTY_SPL_LEAVING_THE_FIELD,
  KICK_OFF_GOAL = PENALTY_SPL_KICK_OFF_GOAL,
  REQUEST_FOR_PICKUP = PENALTY_SPL_REQUEST_FOR_PICKUP,
  COACH_MOTION = PENALTY_SPL_COACH_MOTION,
  SUBSTITUTE = PENALTY_SUBSTITUTE,
  MANUAL = PENALTY_MANUAL
};

/**
 * @brief operator>> is needed for streaming a vector of penalties
 */
inline void operator>>(const Uni::Value& in, Penalty& out)
{
  out = static_cast<Penalty>(in.asInt());
}

/**
 * @brief operator<< is needed for streaming a vector of penalties
 */
inline void operator<<(Uni::Value& out, const Penalty in)
{
  out << static_cast<int>(in);
}

/**
 * @brief GameControllerState is a selection of the data that are provided by the GameController
 * If you need something that is sent by the GameController but not exposed by the GameController module,
 * add it here and make the GameController expose it.
 */


class GameControllerState : public DataType<GameControllerState> {
public:
  /// the packet number (wraparound is accepted)
  unsigned char packetNumber;
  /// the timestamp when the last message has been received
  TimePoint timestampOfLastMessage;
  /// the number of players per team (normally 5)
  unsigned int playersPerTeam;
  /// the type of the game (round robin, playoff or dropin)
  GameType type;
  /// primary game state
  GameState state;
  /// the last time point, when the GameSate was changed
  TimePoint stateChanged;
  /// whether the game is in the first half
  bool firstHalf;
  /// whether the team the robot is in has kickoff
  bool kickoff;
  /// the number of the kick off team (needed for BHULKs message)
  std::uint8_t kickOffTeam;
  /// secondary game state (overtime, penalty shootout)
  SecondaryState secondary;
  // number of seconds shown as secondary time (remaining ready, until free ball, etc)
  float secondaryTime;
  /// the number of the team that caused the last drop in
  unsigned int dropInTeam;
  /// time (seconds) since the last drop in
  float dropInTime;
  /// time (seconds) until the end of the current half
  float remainingTime;
  /// the jersey color of the team the robot is in
  TeamColor teamColor;
  /// number of goals scored by the own team
  unsigned int score;
  /// the current penalty of this robot
  Penalty penalty;
  /// the penalties of all robots in the team (index 0 is player 1)
  std::vector<Penalty> penalties;
  /// time (seconds) until the penalty on this robot is removed
  float remainingPenaltyTime;
  /// whethter the chest button was already pressed in initial - has no meaning when not in initial
  bool chestButtonWasPressedInInitial;
  /**
   * @brief reset could reset the datastructure if it was necessary
   */
  void reset()
  {
    penalties.clear();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["packetNumber"] << packetNumber;
    value["timestampOfLastMessage"] << timestampOfLastMessage;
    value["playersPerTeam"] << playersPerTeam;
    value["type"] << static_cast<int>(type);
    value["state"] << static_cast<int>(state);
    value["stateChanged"] << stateChanged;
    value["firstHalf"] << firstHalf;
    value["kickoff"] << kickoff;
    value["kickOffTeam"] << kickOffTeam;
    value["secondary"] << static_cast<int>(secondary);
    value["secondaryTime"] << secondaryTime;
    value["dropInTeam"] << dropInTeam;
    value["dropInTime"] << dropInTime;
    value["remainingTime"] << remainingTime;
    value["teamColor"] << static_cast<int>(teamColor);
    value["score"] << score;
    value["penalty"] << static_cast<int>(penalty);
    value["penalties"] << penalties;
    value["remainingPenaltyTime"] << remainingPenaltyTime;
    value["chestButtonWasPressedInInitial"] << chestButtonWasPressedInInitial;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    int numberRead = 0;
    value["packetNumber"] >> numberRead;
    packetNumber = numberRead;
    value["timestampOfLastMessage"] >> timestampOfLastMessage;
    value["playersPerTeam"] >> playersPerTeam;
    value["type"] >> numberRead;
    type = static_cast<GameType>(numberRead);
    value["state"] >> numberRead;
    state = static_cast<GameState>(numberRead);
    value["stateChanged"] >> stateChanged;
    value["firstHalf"] >> firstHalf;
    value["kickoff"] >> kickoff;
    value["kickOffTeam"] >> numberRead;
    kickOffTeam = static_cast<std::uint8_t>(numberRead);
    value["secondary"] >> numberRead;
    secondary = static_cast<SecondaryState>(numberRead);
    value["secondaryTime"] >> secondaryTime;
    value["dropInTeam"] >> dropInTeam;
    value["dropInTime"] >> dropInTime;
    value["remainingTime"] >> remainingTime;
    value["teamColor"] >> numberRead;
    teamColor = static_cast<TeamColor>(numberRead);
    value["score"] >> score;
    value["penalty"] >> numberRead;
    penalty = static_cast<Penalty>(numberRead);
    value["penalties"] >> penalties;
    value["remainingPenaltyTime"] >> remainingPenaltyTime;
    value["chestButtonWasPressedInInitial"] >> chestButtonWasPressedInInitial;
  }
};


class RawGameControllerState : public DataType<RawGameControllerState, GameControllerState> {
};
