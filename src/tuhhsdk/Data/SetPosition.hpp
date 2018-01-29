#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"


class SetPosition : public DataType<SetPosition>
{
public:
  /// the position where the robot should be in SET
  Vector2f position;
  /// whether the position is valid
  bool valid;
  /// whether the position is the one nearest to the center circle
  bool isKickoffPosition;
  /**
   * @brief reset invalidates the data type
   */
  void reset()
  {
    valid = false;
    isKickoffPosition = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["position"] << position;
    value["valid"] << valid;
    value["isKickoffPosition"] << isKickoffPosition;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["position"] >> position;
    value["valid"] >> valid;
    value["isKickoffPosition"] >> isKickoffPosition;
  }
};
