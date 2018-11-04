#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"


class BishopPosition : public DataType<BishopPosition>
{
public:
  /// the name of this DataType
  DataTypeName name = "BishopPosition";
  /// whether the bishop position is valid
  bool valid = false;
  /// the position where the robot should be when it has the bishop role
  Vector2f position = Vector2f::Zero();
  /// the orientation of the bishop
  float orientation;
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
