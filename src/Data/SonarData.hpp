#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Definitions.hpp"

class SonarData : public DataType<SonarData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"SonarData"};
  /// filtered values of left and right receivers
  SonarsArray<float> filteredValues = {{0.f, 0.f}};
  /// whether the data is valid
  SonarsArray<bool> valid = {{false, false}};

  /**
   * @brief Overwrite reset to not change previous values, used in filtering.
   */
  void reset() override {}

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["filteredValues"] << filteredValues;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["filteredValues"] >> filteredValues;
    value["valid"] >> valid;
  }
};

class SonarSensorData : public DataType<SonarSensorData>
{
public:
  SonarInfo data;
  SonarsArray<bool> valid = {{false, false}};

  /// the name of this DataType
  DataTypeName name__{"SonarSensorData"};

  void reset() override
  {
    valid.fill(false);
  }
  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["data"] << data;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["data"] >> data;
    value["valid"] >> valid;
  }
};
