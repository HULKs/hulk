#pragma once

#include "Data/RawTeamPlayers.hpp"

struct TeamPlayer : public RawTeamPlayer
{
  TeamPlayer() = default;
  explicit TeamPlayer(const RawTeamPlayer& rTP)
    : RawTeamPlayer(rTP)
  {
  }

  /// whether this player is in the own penalty area
  bool insideOwnPenaltyArea = false;

  void toValue(Uni::Value& value) const override
  {
    RawTeamPlayer::toValue(value);
    value["insideOwnPenaltyArea"] << insideOwnPenaltyArea;
  }

  void fromValue(const Uni::Value& value) override
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
  explicit TeamPlayers(const RawTeamPlayers& rawTeamPlayers)
  {
    players.resize(rawTeamPlayers.rawPlayers.size());
    for (std::size_t i = 0; i < rawTeamPlayers.rawPlayers.size(); i++)
    {
      players[i] = static_cast<TeamPlayer>(rawTeamPlayers.rawPlayers[i]);
    }
    activePlayers = rawTeamPlayers.activePlayers;
    activeHULKPlayers = rawTeamPlayers.activeHULKPlayers;
  }

  void reset() override
  {
    RawTeamPlayers::reset();
    players.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    RawTeamPlayers::toValue(value);
    value["players"] << players;
  }

  void fromValue(const Uni::Value& value) override
  {
    RawTeamPlayers::fromValue(value);
    value["players"] >> players;
  }
};
