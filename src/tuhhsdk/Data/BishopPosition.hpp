#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"


class BishopPosition : public DataType<BishopPosition>
{
public:
  /// whether the bishop position is valid
  bool valid;
  /// the position where the robot should be when it has the bishop role
  Vector2f position;
  /**
   * @brief invalidates the position
   */
  void reset()
  {
    valid = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["valid"] << valid;
    value["position"] << position;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["valid"] >> valid;
    value["position"] >> position;
  }
};
