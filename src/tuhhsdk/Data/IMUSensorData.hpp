#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"


class IMUSensorData : public DataType<IMUSensorData>
{
public:
  /// the accelerometer readings
  Vector3f accelerometer;
  /// the Aldebaran/SoftBank angles. If possible, please use the angles from BodyRotationData.
  Vector3f angle;
  /// the gyroscope readings
  Vector3f gyroscope;
  /**
   * @brief reset does nothing
   */
  void reset()
  {
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["accelerometer"] << accelerometer;
    value["angle"] << angle;
    value["gyroscope"] << gyroscope;
  }
  virtual void fromValue(const Uni::Value& value)
  {
    value["accelerometer"] >> accelerometer;
    value["angle"] >> angle;
    value["gyroscope"] >> gyroscope;
  }
};
