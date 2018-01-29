#pragma once

#include <vector>

#include "Framework/DataType.hpp"


enum class PlayingRole
{
  NONE = 0,
  KEEPER = 1,
  DEFENDER = 2,
  SUPPORT_STRIKER = 3,
  STRIKER = 4,
  BISHOP = 5
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
  /// the role the robot is assigned to
  PlayingRole role;
  /// the roles of all players (playerNumber → role)
  std::vector<PlayingRole> playerRoles;
  /**
   * @brief reset sets the ball to a defined state
   */
  void reset()
  {
    role = PlayingRole::NONE;
    playerRoles.clear();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["role"] << role;
    value["playerRoles"] << playerRoles;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["role"] >> role;
    value["playerRoles"] >> playerRoles;
  }
};
