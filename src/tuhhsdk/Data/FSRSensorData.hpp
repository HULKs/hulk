#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"


class FSRSensorData : public DataType<FSRSensorData>
{
public:
  struct Sensor : public Uni::To, public Uni::From
  {
    /// the weight on the front left sensor
    float frontLeft;
    /// the weight on the front right sensor
    float frontRight;
    /// the weight on the rear left sensor
    float rearLeft;
    /// the weight on the rear right sensor
    float rearRight;
    /// the total weight on the FSR
    float totalWeight;
    /// the center of pressure (should not be used)
    Vector2f cop;

    virtual void toValue(Uni::Value& value) const
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["frontLeft"] << frontLeft;
      value["frontRight"] << frontRight;
      value["rearLeft"] << rearLeft;
      value["rearRight"] << rearRight;
      value["totalWeight"] << totalWeight;
      value["cop"] << cop;
    }
    virtual void fromValue(const Uni::Value& value)
    {
      value["frontLeft"] >> frontLeft;
      value["frontRight"] >> frontRight;
      value["rearLeft"] >> rearLeft;
      value["rearRight"] >> rearRight;
      value["totalWeight"] >> totalWeight;
      value["cop"] >> cop;
    }
  };

  /// sensor data of the left FSR
  Sensor left;
  /// sensor data of the right FSR
  Sensor right;
  /**
   * @brief reset does nothing
   */
  void reset()
  {
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["left"] << left;
    value["right"] << right;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["left"] >> left;
    value["right"] >> right;
  }
};
