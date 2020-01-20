#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"


class SetPosition : public DataType<SetPosition>
{
public:
  /// the name of this DataType
  DataTypeName name = "SetPosition";
  /// the position where the robot should be in SET
  Vector2f position = Vector2f::Zero();
  /// whether the position is valid
  bool valid = false;
  /// whether the position is the one nearest to the center circle
  bool isKickoffPosition;
  /**
   * @brief reset invalidates the data type
   */
  void reset() override
  {
    isKickoffPosition = false;
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["position"] << position;
    value["valid"] << valid;
    value["isKickoffPosition"] << isKickoffPosition;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["position"] >> position;
    value["valid"] >> valid;
    value["isKickoffPosition"] >> isKickoffPosition;
  }
};
