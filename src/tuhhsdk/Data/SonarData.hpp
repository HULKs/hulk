#pragma once

#include "Definitions/keys.h"
#include "Framework/DataType.hpp"


class SonarData : public DataType<SonarData>
{
public:
  /// the name of this DataType
  DataTypeName name = "SonarData";
  /// filtered values of left and right receivers
  std::array<float, 2> filteredValues = {{0, 0}};
  /// whether the data is valid
  std::array<bool, 2> valid = {{false, false}};

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
  std::array<float, keys::sensor::SONAR_MAX> data;
  std::array<bool, keys::sensor::SONAR_MAX> valid;


  /// the name of this DataType
  DataTypeName name = "SonarSensorData";

  void reset() override
  {
    data.fill(0);
    valid.fill(false);
  }
  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["SONAR_ACTUATOR"] << data[keys::sensor::SONAR_ACTUATOR];
    value["SONAR_SENSOR"] << data[keys::sensor::SONAR_SENSOR];
    value["SONAR_LEFT_SENSOR_0"] << data[keys::sensor::SONAR_LEFT_SENSOR_0];
    value["SONAR_LEFT_SENSOR_1"] << data[keys::sensor::SONAR_LEFT_SENSOR_1];
    value["SONAR_LEFT_SENSOR_2"] << data[keys::sensor::SONAR_LEFT_SENSOR_2];
    value["SONAR_LEFT_SENSOR_3"] << data[keys::sensor::SONAR_LEFT_SENSOR_3];
    value["SONAR_LEFT_SENSOR_4"] << data[keys::sensor::SONAR_LEFT_SENSOR_4];
    value["SONAR_LEFT_SENSOR_5"] << data[keys::sensor::SONAR_LEFT_SENSOR_5];
    value["SONAR_LEFT_SENSOR_6"] << data[keys::sensor::SONAR_LEFT_SENSOR_6];
    value["SONAR_LEFT_SENSOR_7"] << data[keys::sensor::SONAR_LEFT_SENSOR_7];
    value["SONAR_LEFT_SENSOR_8"] << data[keys::sensor::SONAR_LEFT_SENSOR_8];
    value["SONAR_LEFT_SENSOR_9"] << data[keys::sensor::SONAR_LEFT_SENSOR_9];
    value["SONAR_RIGHT_SENSOR_0"] << data[keys::sensor::SONAR_RIGHT_SENSOR_0];
    value["SONAR_RIGHT_SENSOR_1"] << data[keys::sensor::SONAR_RIGHT_SENSOR_1];
    value["SONAR_RIGHT_SENSOR_2"] << data[keys::sensor::SONAR_RIGHT_SENSOR_2];
    value["SONAR_RIGHT_SENSOR_3"] << data[keys::sensor::SONAR_RIGHT_SENSOR_3];
    value["SONAR_RIGHT_SENSOR_4"] << data[keys::sensor::SONAR_RIGHT_SENSOR_4];
    value["SONAR_RIGHT_SENSOR_5"] << data[keys::sensor::SONAR_RIGHT_SENSOR_5];
    value["SONAR_RIGHT_SENSOR_6"] << data[keys::sensor::SONAR_RIGHT_SENSOR_6];
    value["SONAR_RIGHT_SENSOR_7"] << data[keys::sensor::SONAR_RIGHT_SENSOR_7];
    value["SONAR_RIGHT_SENSOR_8"] << data[keys::sensor::SONAR_RIGHT_SENSOR_8];
    value["SONAR_RIGHT_SENSOR_9"] << data[keys::sensor::SONAR_RIGHT_SENSOR_9];

    value["valid_SONAR_ACTUATOR"] << valid[keys::sensor::SONAR_ACTUATOR];
    value["valid_SONAR_SENSOR"] << valid[keys::sensor::SONAR_SENSOR];
    value["valid_SONAR_LEFT_SENSOR_0"] << valid[keys::sensor::SONAR_LEFT_SENSOR_0];
    value["valid_SONAR_LEFT_SENSOR_1"] << valid[keys::sensor::SONAR_LEFT_SENSOR_1];
    value["valid_SONAR_LEFT_SENSOR_2"] << valid[keys::sensor::SONAR_LEFT_SENSOR_2];
    value["valid_SONAR_LEFT_SENSOR_3"] << valid[keys::sensor::SONAR_LEFT_SENSOR_3];
    value["valid_SONAR_LEFT_SENSOR_4"] << valid[keys::sensor::SONAR_LEFT_SENSOR_4];
    value["valid_SONAR_LEFT_SENSOR_5"] << valid[keys::sensor::SONAR_LEFT_SENSOR_5];
    value["valid_SONAR_LEFT_SENSOR_6"] << valid[keys::sensor::SONAR_LEFT_SENSOR_6];
    value["valid_SONAR_LEFT_SENSOR_7"] << valid[keys::sensor::SONAR_LEFT_SENSOR_7];
    value["valid_SONAR_LEFT_SENSOR_8"] << valid[keys::sensor::SONAR_LEFT_SENSOR_8];
    value["valid_SONAR_LEFT_SENSOR_9"] << valid[keys::sensor::SONAR_LEFT_SENSOR_9];
    value["valid_SONAR_RIGHT_SENSOR_0"] << valid[keys::sensor::SONAR_RIGHT_SENSOR_0];
    value["valid_SONAR_RIGHT_SENSOR_1"] << valid[keys::sensor::SONAR_RIGHT_SENSOR_1];
    value["valid_SONAR_RIGHT_SENSOR_2"] << valid[keys::sensor::SONAR_RIGHT_SENSOR_2];
    value["valid_SONAR_RIGHT_SENSOR_3"] << valid[keys::sensor::SONAR_RIGHT_SENSOR_3];
    value["valid_SONAR_RIGHT_SENSOR_4"] << valid[keys::sensor::SONAR_RIGHT_SENSOR_4];
    value["valid_SONAR_RIGHT_SENSOR_5"] << valid[keys::sensor::SONAR_RIGHT_SENSOR_5];
    value["valid_SONAR_RIGHT_SENSOR_6"] << valid[keys::sensor::SONAR_RIGHT_SENSOR_6];
    value["valid_SONAR_RIGHT_SENSOR_7"] << valid[keys::sensor::SONAR_RIGHT_SENSOR_7];
    value["valid_SONAR_RIGHT_SENSOR_8"] << valid[keys::sensor::SONAR_RIGHT_SENSOR_8];
    value["valid_SONAR_RIGHT_SENSOR_9"] << valid[keys::sensor::SONAR_RIGHT_SENSOR_9];
  }

  void fromValue(const Uni::Value& value) override
  {
    value["SONAR_ACTUATOR"] >> data[keys::sensor::SONAR_ACTUATOR];
    value["SONAR_SENSOR"] >> data[keys::sensor::SONAR_SENSOR];
    value["SONAR_LEFT_SENSOR_0"] >> data[keys::sensor::SONAR_LEFT_SENSOR_0];
    value["SONAR_LEFT_SENSOR_1"] >> data[keys::sensor::SONAR_LEFT_SENSOR_1];
    value["SONAR_LEFT_SENSOR_2"] >> data[keys::sensor::SONAR_LEFT_SENSOR_2];
    value["SONAR_LEFT_SENSOR_3"] >> data[keys::sensor::SONAR_LEFT_SENSOR_3];
    value["SONAR_LEFT_SENSOR_4"] >> data[keys::sensor::SONAR_LEFT_SENSOR_4];
    value["SONAR_LEFT_SENSOR_5"] >> data[keys::sensor::SONAR_LEFT_SENSOR_5];
    value["SONAR_LEFT_SENSOR_6"] >> data[keys::sensor::SONAR_LEFT_SENSOR_6];
    value["SONAR_LEFT_SENSOR_7"] >> data[keys::sensor::SONAR_LEFT_SENSOR_7];
    value["SONAR_LEFT_SENSOR_8"] >> data[keys::sensor::SONAR_LEFT_SENSOR_8];
    value["SONAR_LEFT_SENSOR_9"] >> data[keys::sensor::SONAR_LEFT_SENSOR_9];
    value["SONAR_RIGHT_SENSOR_0"] >> data[keys::sensor::SONAR_RIGHT_SENSOR_0];
    value["SONAR_RIGHT_SENSOR_1"] >> data[keys::sensor::SONAR_RIGHT_SENSOR_1];
    value["SONAR_RIGHT_SENSOR_2"] >> data[keys::sensor::SONAR_RIGHT_SENSOR_2];
    value["SONAR_RIGHT_SENSOR_3"] >> data[keys::sensor::SONAR_RIGHT_SENSOR_3];
    value["SONAR_RIGHT_SENSOR_4"] >> data[keys::sensor::SONAR_RIGHT_SENSOR_4];
    value["SONAR_RIGHT_SENSOR_5"] >> data[keys::sensor::SONAR_RIGHT_SENSOR_5];
    value["SONAR_RIGHT_SENSOR_6"] >> data[keys::sensor::SONAR_RIGHT_SENSOR_6];
    value["SONAR_RIGHT_SENSOR_7"] >> data[keys::sensor::SONAR_RIGHT_SENSOR_7];
    value["SONAR_RIGHT_SENSOR_8"] >> data[keys::sensor::SONAR_RIGHT_SENSOR_8];
    value["SONAR_RIGHT_SENSOR_9"] >> data[keys::sensor::SONAR_RIGHT_SENSOR_9];

    value["valid_SONAR_ACTUATOR"] >> valid[keys::sensor::SONAR_ACTUATOR];
    value["valid_SONAR_SENSOR"] >> valid[keys::sensor::SONAR_SENSOR];
    value["valid_SONAR_LEFT_SENSOR_0"] >> valid[keys::sensor::SONAR_LEFT_SENSOR_0];
    value["valid_SONAR_LEFT_SENSOR_1"] >> valid[keys::sensor::SONAR_LEFT_SENSOR_1];
    value["valid_SONAR_LEFT_SENSOR_2"] >> valid[keys::sensor::SONAR_LEFT_SENSOR_2];
    value["valid_SONAR_LEFT_SENSOR_3"] >> valid[keys::sensor::SONAR_LEFT_SENSOR_3];
    value["valid_SONAR_LEFT_SENSOR_4"] >> valid[keys::sensor::SONAR_LEFT_SENSOR_4];
    value["valid_SONAR_LEFT_SENSOR_5"] >> valid[keys::sensor::SONAR_LEFT_SENSOR_5];
    value["valid_SONAR_LEFT_SENSOR_6"] >> valid[keys::sensor::SONAR_LEFT_SENSOR_6];
    value["valid_SONAR_LEFT_SENSOR_7"] >> valid[keys::sensor::SONAR_LEFT_SENSOR_7];
    value["valid_SONAR_LEFT_SENSOR_8"] >> valid[keys::sensor::SONAR_LEFT_SENSOR_8];
    value["valid_SONAR_LEFT_SENSOR_9"] >> valid[keys::sensor::SONAR_LEFT_SENSOR_9];
    value["valid_SONAR_RIGHT_SENSOR_0"] >> valid[keys::sensor::SONAR_RIGHT_SENSOR_0];
    value["valid_SONAR_RIGHT_SENSOR_1"] >> valid[keys::sensor::SONAR_RIGHT_SENSOR_1];
    value["valid_SONAR_RIGHT_SENSOR_2"] >> valid[keys::sensor::SONAR_RIGHT_SENSOR_2];
    value["valid_SONAR_RIGHT_SENSOR_3"] >> valid[keys::sensor::SONAR_RIGHT_SENSOR_3];
    value["valid_SONAR_RIGHT_SENSOR_4"] >> valid[keys::sensor::SONAR_RIGHT_SENSOR_4];
    value["valid_SONAR_RIGHT_SENSOR_5"] >> valid[keys::sensor::SONAR_RIGHT_SENSOR_5];
    value["valid_SONAR_RIGHT_SENSOR_6"] >> valid[keys::sensor::SONAR_RIGHT_SENSOR_6];
    value["valid_SONAR_RIGHT_SENSOR_7"] >> valid[keys::sensor::SONAR_RIGHT_SENSOR_7];
    value["valid_SONAR_RIGHT_SENSOR_8"] >> valid[keys::sensor::SONAR_RIGHT_SENSOR_8];
    value["valid_SONAR_RIGHT_SENSOR_9"] >> valid[keys::sensor::SONAR_RIGHT_SENSOR_9];
  }
};
