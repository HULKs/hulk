#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Definitions.hpp"
#include "Tools/Math/Eigen.hpp"


class FSRSensorData : public DataType<FSRSensorData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"FSRSensorData"};
  /// sensor data of the left FSR
  FSRInfo leftFoot;
  /// the summed up pressure measurements of the left foot
  float totalLeft;
  /// sensor data of the right FSR
  FSRInfo rightFoot;
  /// the summed up pressure measurements of the right foot
  float totalRight;
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
    value["leftFoot"] << leftFoot;
    value["totalLeft"] << totalLeft;
    value["rightFoot"] << rightFoot;
    value["totalRight"] << totalRight;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["leftFoot"] >> leftFoot;
    value["totalLeft"] >> totalLeft;
    value["rightFoot"] >> rightFoot;
    value["totalRight"] >> totalRight;
    value["valid"] >> valid;
  }
};
