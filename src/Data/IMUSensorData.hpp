#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"


class IMUSensorData : public DataType<IMUSensorData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"IMUSensorData"};
  /// the accelerometer readings
  Vector3f accelerometer = Vector3f::Zero();
  /// the Aldebaran/SoftBank angles. If possible, please use the angles from BodyRotationData.
  Vector2f angle = Vector2f::Zero();
  /// the gyroscope readings
  Vector3f gyroscope = Vector3f::Zero();
  /// whether the content is valid
  bool valid = false;
  /**
   * @brief marks the content as invalid
   */
  void reset() override
  {
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["accelerometer"] << accelerometer;
    value["angle"] << angle;
    value["gyroscope"] << gyroscope;
    value["valid"] << valid;
  }
  void fromValue(const Uni::Value& value) override
  {
    value["accelerometer"] >> accelerometer;
    value["angle"] >> angle;
    value["gyroscope"] >> gyroscope;
    value["valid"] >> valid;
  }
};
