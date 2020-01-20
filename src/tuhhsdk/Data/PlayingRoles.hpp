#pragma once

#include <vector>

#include "Framework/DataType.hpp"


/// Definition of playing roles. If one changes this, one alsa has to change the BHULKs Role enum
/// and the BHULKsHelper
enum class PlayingRole
{
  NONE = 0,
  KEEPER = 1,
  DEFENDER_LEFT = 2,
  DEFENDER_RIGHT = 3,
  SUPPORT_STRIKER = 4,
  STRIKER = 5,
  BISHOP = 6,
  REPLACEMENT_KEEPER = 7
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
  DataTypeName name = "PlayingRoles";
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
