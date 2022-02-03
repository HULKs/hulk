#pragma once

#include <vector>

#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Math/Eigen.hpp"

class RobotData : public DataType<RobotData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"RobotData"};
  /// a vector of detected robots in robot coordinates
  VecVector2f positions;
  /// the timestamp of the image these detections were derived from
  Clock::time_point timestamp;
  /**
   * @brief reset resets this datatype by clearing the position vector
   */
  void reset() override
  {
    positions.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["positions"] << positions;
    value["timestamp"] << timestamp;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["positions"] >> positions;
    value["timestamp"] >> timestamp;
  }
};
