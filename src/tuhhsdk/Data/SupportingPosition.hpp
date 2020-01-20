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
  void reset() override
  {
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["valid"] << valid;
    value["position"] << position;
    value["orientation"] << orientation;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["valid"] >> valid;
    value["position"] >> position;
    value["orientation"] >> orientation;
  }
};
