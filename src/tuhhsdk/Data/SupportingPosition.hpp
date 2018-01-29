#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"


class SupportingPosition : public DataType<SupportingPosition>
{
public:
  /// whether the supporting position is valid
  bool valid;
  /// the position where the robot should be when it has the support striker role
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
