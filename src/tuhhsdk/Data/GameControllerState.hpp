#pragma once

#include <vector>

#include "Definitions/RoboCupGameControlData.h"
#include "Framework/DataType.hpp"
#include "Tools/Time.hpp"


enum class CompetitionPhase
{
  ROUNDROBIN = COMPETITION_PHASE_ROUNDROBIN,
  PLAYOFF = COMPETITION_PHASE_PLAYOFF
};

enum class CompetitionType
{
  NORMAL = COMPETITION_TYPE_NORMAL,
  MIXED_TEAM = COMPETITION_TYPE_MIXEDTEAM,
  GENERAL_PENALTY_KICK = COMPETITION_TYPE_GENERAL_PENALTY_KICK
};

enum class SetPlay
{
  NONE = SET_PLAY_NONE,
  GOAL_FREE_KICK = SET_PLAY_GOAL_FREE_KICK,
  PUSHING_FREE_KICK = SET_PLAY_PUSHING_FREE_KICK
};

enum class GameState
{
  INITIAL = STATE_INITIAL,
  READY = STATE_READY,
  SET = STATE_SET,
  PLAYING = STATE_PLAYING,
  FINISHED = STATE_FINISHED
};

enum class GamePhase
{
  NORMAL = GAME_PHASE_NORMAL,
  PENALTYSHOOT = GAME_PHASE_PENALTYSHOOT,
  OVERTIME = GAME_PHASE_OVERTIME,
  TIMEOUT = GAME_PHASE_TIMEOUT
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
  LOCAL_GAME_STUCK = PENALTY_SPL_LOCAL_GAME_STUCK,
  SUBSTITUTE = PENALTY_SUBSTITUTE,
  MANUAL = PENALTY_MANUAL
};

/**
 * @brief operator>> is needed for streaming a vector of penalties
 */
inline void operator>>(const Uni::Value& in, Penalty& out)
{
  out = static_cast<Penalty>(in.asInt32());
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
 * If you need something that is sent by the GameController but not exposed by the GameController
 * module, add it here and make the GameController expose it.
 */


class GameControllerState : public DataType<GameControllerState>
{
public:
  /// the name of this DataType
  DataTypeName name = "GameControllerState";
  /// the packet number (wraparound is accepted)
  unsigned char packetNumber = 0;
  /// the timestamp when the last message has been received
  TimePoint timestampOfLastMessage;
  /// the number of players per team (normally 5)
  unsigned int playersPerTeam = 0;
  /// the type of the competition (Normal, MixedTeam, GeneralPenaltyKick)
  CompetitionType type = CompetitionType::NORMAL;
  /// the phase of the competition (Roundrobin, Playoff)
  CompetitionPhase competitionPhase = CompetitionPhase::PLAYOFF;
  /// primary game state
  GameState gameState = GameState::INITIAL;
  /// the last time point when the GameState was changed
  TimePoint gameStateChanged;
  /// game phase (normal, overtime, penalty shootout, timeout)
  GamePhase gamePhase = GamePhase::NORMAL;
  /// the active play set (none, goal free kick, pushing free kick)
  SetPlay setPlay = SetPlay::NONE;
  /// the last time point when SetPlay was changed
  TimePoint setPlayChanged;
  /// whether the game is in the first half
  bool firstHalf = true;
  /// whether our team is the kicking team (during freeKick or when SET changes to PLAYING)
  bool kickingTeam = false;
  /// the number of the kicking team (needed for BHULKs message)
  uint8_t kickingTeamNumber = 0;
  /// number of seconds shown as secondary time (remaining ready, until free ball, etc)
  float secondaryTime = 0.f;
  /// the number of the team that caused the last drop in
  unsigned int dropInTeam = 0;
  /// time (seconds) since the last drop in
  float dropInTime = 0.f;
  /// time (seconds) until the end of the current half
  float remainingTime = 10.f * 60.f;
  /// the jersey color of the team the robot is in
  TeamColor teamColor = TeamColor::GRAY;
  /// number of goals scored by the own team
  unsigned int score = 0;
  /// the current penalty of this robot
  Penalty penalty = Penalty::NONE;
  /// the penalties of all robots in the team (index 0 is player 1)
  std::vector<Penalty> penalties;
  /// time (seconds) until the penalty on this robot is removed
  float remainingPenaltyTime = 0.f;
  /// whethter the chest button was already pressed in initial - has no meaning when not in initial
  bool chestButtonWasPressedInInitial = false;
  /// whether the content is valid
  bool valid = false;
  /**
   * @brief reset could reset the datastructure if it was necessary
   */
  void reset() override
  {
    penalties.clear();
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["packetNumber"] << packetNumber;
    value["timestampOfLastMessage"] << timestampOfLastMessage;
    value["playersPerTeam"] << playersPerTeam;
    value["type"] << static_cast<int>(type);
    value["competitionPhase"] << static_cast<int>(competitionPhase);
    value["gameState"] << static_cast<int>(gameState);
    value["gameStateChanged"] << gameStateChanged;
    value["gamePhase"] << static_cast<int>(gamePhase);
    value["setPlay"] << static_cast<int>(setPlay);
    value["setPlayChanged"] << setPlayChanged;
    value["firstHalf"] << firstHalf;
    value["kickingTeam"] << kickingTeam;
    value["kickingTeamNumber"] << kickingTeamNumber;
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
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    int numberRead = 0;
    value["packetNumber"] >> numberRead;
    packetNumber = static_cast<unsigned char>(numberRead);
    value["timestampOfLastMessage"] >> timestampOfLastMessage;
    value["playersPerTeam"] >> playersPerTeam;
    value["type"] >> numberRead;
    type = static_cast<CompetitionType>(numberRead);
    value["competitionPhase"] >> numberRead;
    competitionPhase = static_cast<CompetitionPhase>(numberRead);
    value["gameState"] >> numberRead;
    gameState = static_cast<GameState>(numberRead);
    value["gameStateChanged"] >> gameStateChanged;
    value["gamePhase"] >> numberRead;
    gamePhase = static_cast<GamePhase>(numberRead);
    value["setPlay"] >> numberRead;
    setPlay = static_cast<SetPlay>(numberRead);
    value["setPlayChanged"] >> setPlayChanged;
    value["firstHalf"] >> firstHalf;
    value["kickingTeam"] >> kickingTeam;
    value["kickingTeamNumber"] >> numberRead;
    kickingTeamNumber = static_cast<uint8_t>(numberRead);
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
    value["valid"] >> valid;
  }
};


class RawGameControllerState : public DataType<RawGameControllerState, GameControllerState>
{
public:
  /// the name of this DataType
  DataTypeName name = "RawGameControllerState";
};
