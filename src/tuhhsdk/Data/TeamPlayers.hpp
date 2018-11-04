#pragma once

#include "Data/RawTeamPlayers.hpp"

struct TeamPlayer : public RawTeamPlayer
{
  TeamPlayer() = default;
  TeamPlayer(const RawTeamPlayer& rTP)
    : RawTeamPlayer(rTP)
  {
  }

  /// whether this player is in the own penalty area
  bool insideOwnPenaltyArea = false;

  virtual void toValue(Uni::Value& value) const
  {
    RawTeamPlayer::toValue(value);
    value["insideOwnPenaltyArea"] << insideOwnPenaltyArea;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    RawTeamPlayer::fromValue(value);
    value["insideOwnPenaltyArea"] >> insideOwnPenaltyArea;
  }
};

class TeamPlayers : public DataType<TeamPlayers, RawTeamPlayers>
{
public:
  DataTypeName name = "TeamPlayers";
  std::vector<TeamPlayer> players;

  TeamPlayers() = default;
  TeamPlayers(const RawTeamPlayers& rawTeamPlayers)
  {
    players.resize(rawTeamPlayers.rawPlayers.size());
    for (std::size_t i = 0; i < rawTeamPlayers.rawPlayers.size(); i++)
    {
      players[i] = rawTeamPlayers.rawPlayers[i];
    }
    activePlayers = rawTeamPlayers.activePlayers;
    activeHULKPlayers = rawTeamPlayers.activeHULKPlayers;
  }

  void reset()
  {
    RawTeamPlayers::reset();
    players.clear();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    RawTeamPlayers::toValue(value);
    value["players"] << players;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    RawTeamPlayers::fromValue(value);
    value["players"] >> players;
  }
};
