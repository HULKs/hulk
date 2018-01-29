#pragma once

#include "Framework/DataType.hpp"

class SonarData : public DataType<SonarData>
{
public:
  /// sonar value of left receiver
  float sonarLeft;
  /// sonar value of right receiver
  float sonarRight;

  /**
   * @brief reset values to invalid value.
   */
  void reset()
  {
    sonarLeft = -1;
    sonarRight = -1;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["sonarLeft"] << sonarLeft;
    value["sonarRight"] << sonarRight;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["sonarLeft"] >> sonarLeft;
    value["sonarRight"] >> sonarRight;
  }
};

class SonarSensorData : public DataType<SonarSensorData, SonarData>
{
};
