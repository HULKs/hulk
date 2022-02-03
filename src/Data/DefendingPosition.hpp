#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"


class DefendingPosition : public DataType<DefendingPosition>
{
public:
  /// the name of this DataType
  DataTypeName name__{"DefendingPosition"};
  /// whether the playing position is valid
  bool valid = false;
  /// the position where the robot should be when it has the defender role
  Vector2f position = Vector2f::Zero();
  /**
   * @brief invalidates the position
   */
  void reset() override
  {
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["valid"] << valid;
    value["position"] << position;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["valid"] >> valid;
    value["position"] >> position;
  }
};
