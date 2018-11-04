#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"


class SupportingPosition : public DataType<SupportingPosition>
{
public:
  /// the name of this DataType
  DataTypeName name = "SupportingPosition";
  /// whether the supporting position is valid
  bool valid = false;
  /// the position where the robot should be when it has the support striker role
  Vector2f position = Vector2f::Zero();
  /// the desired orientation of the support striker
  float orientation = 0.f;
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
    value["orientation"] << orientation;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["valid"] >> valid;
    value["position"] >> position;
    value["orientation"] >> orientation;
  }
};
