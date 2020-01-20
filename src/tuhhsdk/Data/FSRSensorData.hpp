#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"


class FSRSensorData : public DataType<FSRSensorData>
{
public:
  /// the name of this DataType
  DataTypeName name = "FSRSensorData";
  struct Sensor : public Uni::To, public Uni::From
  {
    /// the weight on the front left sensor
    float frontLeft = 0.f;
    /// the weight on the front right sensor
    float frontRight = 0.f;
    /// the weight on the rear left sensor
    float rearLeft = 0.f;
    /// the weight on the rear right sensor
    float rearRight = 0.f;
    /// the total weight on the FSR
    float totalWeight = 0.f;
    /// the center of pressure (should not be used)
    Vector2f cop = Vector2f::Zero();

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["frontLeft"] << frontLeft;
      value["frontRight"] << frontRight;
      value["rearLeft"] << rearLeft;
      value["rearRight"] << rearRight;
      value["totalWeight"] << totalWeight;
      value["cop"] << cop;
    }
    void fromValue(const Uni::Value& value) override
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
    value["left"] << left;
    value["right"] << right;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["left"] >> left;
    value["right"] >> right;
    value["valid"] >> valid;
  }
};
