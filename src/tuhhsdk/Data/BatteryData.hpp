#pragma once

#include "Framework/DataType.hpp"


class BatteryData : public DataType<BatteryData>
{
public:
  /// the name of this DataType
  DataTypeName name = "BatteryData";
  /// charge level in [0,1]
  float charge;
  /// battery current (positive is charging) in Ampere
  float current;
  /// temperature (in percent, whatever that means)
  float temperature;
  /// battery status (there is no documentation for this value)
  float status;
  /**
   * @brief reset does nothing
   */
  void reset() override
  {
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["charge"] << charge;
    value["current"] << current;
    value["temperature"] << temperature;
    value["status"] << status;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["charge"] >> charge;
    value["current"] >> current;
    value["temperature"] >> temperature;
    value["status"] >> status;
  }
};
