#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Definitions.hpp"
#include <array>

class JointCalibrationData : public DataType<JointCalibrationData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"JointCalibrationData"};
  /// the offset of all joints in rad (is added to the final joint values and subtracted from the
  /// measurements)
  JointsArray<float> calibrationOffsets{};

  void reset() override
  {
    // This is empty on purpose since the calibration provider does not rewrite the offsets every
    // cycle
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["calibrationOffsets"] << calibrationOffsets;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["calibrationOffsets"] >> calibrationOffsets;
  }
};
