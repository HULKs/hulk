#pragma once

#include <vector>

#include "Framework/DataType.hpp"


/// Definition of playing roles. If one changes this, one also has to change the
/// DevilSmashStandardMessage and the DSHelper
enum class PlayingRole
{
  NONE = 0,
  KEEPER = 1,
  DEFENDER = 2,
  SUPPORT_STRIKER = 3,
  STRIKER = 4,
  BISHOP = 5,
  REPLACEMENT_KEEPER = 6,
  LOSER = 7,
  SEARCHER = 8
};

inline void operator>>(const Uni::Value& in, PlayingRole& out)
{
  int readValue;
  in >> readValue;
  out = static_cast<PlayingRole>(readValue);
}

inline void operator<<(Uni::Value& out, const PlayingRole& in)
{
  out << static_cast<int>(in);
}

class PlayingRoles : public DataType<PlayingRoles>
{
public:
  /// the name of this DataType
  DataTypeName name__{"PlayingRoles"};
  /// the role the robot is assigned to
  PlayingRole role = PlayingRole::NONE;
  /// the roles of all players (playerNumber-1 → role)
  std::vector<PlayingRole> playerRoles;

  void reset() override
  {
    role = PlayingRole::NONE;
    playerRoles.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["role"] << role;
    value["playerRoles"] << playerRoles;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["role"] >> role;
    value["playerRoles"] >> playerRoles;
  }
};
